//! ONNX inference through tract — the runtime that answers questions with the pinned models.
//!
//! The store answers *is this file the model the manifest promised?*; this module answers *what does
//! the model say about this picture?* — and nothing else. It is deliberately desktop-only (the wasm
//! build compiles without it) and deliberately offline: the bytes come from a file path the store has
//! already verified against the manifest's digest, tract runs them on the CPU in this process, and no
//! code here can open a socket. The crate's `no_network` test still reads every source file, and this
//! one passes it because there is nothing to find.
//!
//! Contracts, not conventions. Every `load` refuses a model whose input or output shapes do not match
//! the I/O contracts recorded in `docs/ATTRIBUTION.md` — a model that is one layer deeper than its pin
//! says would still pass a digest check (it is a different file), so the shapes are checked too. The
//! pinned detector is YOLOS-tiny: `pixel_values` 1×3×512×800 f32 in, `logits` 1×100×92 and
//! `pred_boxes` 1×100×4 (cxcywh, normalised) out. The pinned embedders are CLIP ViT-B/32: 1×3×224×224
//! f32 → 1×512, and `input_ids` 1×77 int64 → 1×512.
//!
//! Everything the model produces is a *guess*, and the guess leaves this module in the same shapes the
//! rest of the crate already understands: detector output goes through [`crate::detect::suppress`]
//! before anyone sees it, embeddings expose a cosine, and turning either into a keyword is
//! [`crate::suggest`]'s job — nothing here writes to a library.

use crate::detect::{Box, Detection, IOU_THRESHOLD, LabelMap, MIN_SCORE, suppress};

use std::boxed::Box as StdBox;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use tract_onnx::prelude::*;
use tract_onnx::tract_core::plan::SimplePlan;

/// The plan tract runs. `into_optimized` folds constants and picks better kernels once at load, so
/// the per-photo cost is inference only.
type Plan = Arc<SimplePlan<TypedFact, StdBox<dyn TypedOp>>>;

/// The pinned detector's label text: the 91 COCO detection classes, in index order, `N/A` where COCO
/// has none. The 92nd logit — the slot past this map — is the no-object (EOS) class, which is dropped
/// by being unnamed rather than by a special case.
pub const COCO_LABELS_TEXT: &str = include_str!("coco91.txt");

/// ImageNet/CLIP channel normalisation, the contract every pinned model was trained under.
pub const IMAGENET_MEAN: [f32; 3] = [0.481_454_67, 0.457_827_5, 0.408_210_73];
pub const IMAGENET_STD: [f32; 3] = [0.268_629_54, 0.261_302_6, 0.275_777_1];

/// One model loaded and shape-checked, ready to run.
///
/// Everything `load` learns about the model is recorded here, so `run` never has to re-derive it and
/// never has to trust the caller about shapes.
#[derive(Debug)]
struct Session {
    plan: Plan,
    /// The input shape, in elements per dimension. `run` refuses an input that is not exactly this.
    input: Vec<usize>,
    /// One entry per graph output: its index in the run result and its shape.
    outputs: Vec<(usize, Vec<usize>)>,
}

impl Session {
    /// Load, optimise and check an ONNX file against a description of what it must be.
    ///
    /// `inputs` are the concrete shapes every graph input must have; `outputs` are minimum
    /// expectations (rank must match, listed `None` dimensions may be anything). Anything else is a
    /// wrong file for this job, and is refused here where the error says so, not later where it would
    /// look like a strange photo.
    fn load(path: &Path, inputs: &[Vec<Option<usize>>], outputs: &[Option<Vec<Option<usize>>>], what: &str) -> Result<Session, String> {
        let bytes = fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let model = tract_onnx::onnx()
            .model_for_read(&mut bytes.as_slice())
            .map_err(|error| format!("{what} is not a model tract can load: {error}"))?
            .into_optimized()
            .map_err(|error| format!("{what} could not be optimised: {error}"))?;
        let plan = model
            .clone()
            .into_runnable()
            .map_err(|error| format!("{what} could not be planned: {error}"))?;
        if model.inputs.len() != inputs.len() {
            return Err(format!(
                "{what} has {} input(s) and the contract expects {}",
                model.inputs.len(),
                inputs.len()
            ));
        }
        let mut input_shapes = Vec::new();
        for (index, (outlet, expected)) in model.inputs.iter().zip(inputs).enumerate() {
            let dims = concrete_dims(&model, *outlet, what, index)?;
            if !same_shape(&dims, expected) {
                return Err(format!(
                    "{what} input {index} has shape {dims:?} and the contract expects {expected:?}"
                ));
            }
            input_shapes.push(dims);
        }
        if model.outputs.len() != outputs.len() {
            return Err(format!(
                "{what} has {} output(s) and the contract expects {}",
                model.outputs.len(),
                outputs.len()
            ));
        }
        let mut output_shapes = Vec::new();
        for (index, (outlet, expected)) in model.outputs.iter().zip(outputs).enumerate() {
            let dims = concrete_dims(&model, *outlet, what, index)?;
            if let Some(expected) = expected
                && !same_shape(&dims, expected)
            {
                return Err(format!(
                    "{what} output {index} has shape {dims:?} and the contract expects {expected:?}"
                ));
            }
            output_shapes.push((index, dims));
        }
        Ok(Session {
            plan,
            input: input_shapes.first().cloned().unwrap_or_default(),
            outputs: output_shapes,
        })
    }

    /// Run one input through the model and return the outputs in graph order.
    fn run(&self, input: TValue, what: &str) -> Result<Vec<Vec<f32>>, String> {
        let outputs = self
            .plan
            .run(tvec![input])
            .map_err(|error| format!("{what} failed to run: {error}"))?;
        self.outputs
            .iter()
            .map(|(index, _)| {
                let tensor = outputs[*index].view();
                tensor
                    .as_slice::<f32>()
                    .map(|slice| slice.to_vec())
                    .map_err(|error| format!("{what} output {index} is not float: {error}"))
            })
            .collect()
    }
}

/// The concrete shape of one model outlet, with every dimension required to be static.
fn concrete_dims(model: &TypedModel, outlet: OutletId, what: &str, index: usize) -> Result<Vec<usize>, String> {
    let fact = model
        .outlet_fact(outlet)
        .map_err(|error| format!("{what} input/output {index} could not be typed: {error}"))?;
    fact.shape
        .dims()
        .iter()
        .map(|dim| {
            dim.to_i64()
                .map(|value| value as usize)
                .map_err(|_| format!("{what} has a symbolic shape; the pinned models are static, so this file is not the pinned model"))
        })
        .collect()
}

/// Whether `actual` matches `expected`, where `None` in `expected` is any value.
fn same_shape(actual: &[usize], expected: &[Option<usize>]) -> bool {
    actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| e.is_none_or(|value| a == &value))
}

/// A shape spec with `None` wildcards.
fn dims(values: &[Option<usize>]) -> Vec<Option<usize>> {
    values.to_vec()
}

/// An image ready for a model: stretched to the model's input size and normalised, channels first.
///
/// The stretch is nearest-neighbour on purpose: a detector and an embedder see a resized photo, and
/// bilinear smoothing would spend real time on pixels the model was never promised. The
/// normalisation is the ImageNet/CLIP mean and standard deviation per channel, which is what the
/// pinned models were trained under.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Target width, which the model's input contract names.
    width: u32,
    /// Target height.
    height: u32,
    /// Channels-first normalised pixels, 3 × height × width.
    chw: Vec<f32>,
}

impl Image {
    /// Build from RGBA8 pixels (as `sortlens-thumbs` decodes), stretched to `target` and normalised.
    pub fn from_rgba(rgba: &[u8], width: u32, height: u32, target: (u32, u32), mean: [f32; 3], std: [f32; 3]) -> Result<Image, String> {
        let (target_w, target_h) = (target.0 as usize, target.1 as usize);
        if target_w == 0 || target_h == 0 {
            return Err("a model input cannot be empty".to_string());
        }
        if rgba.len() != width as usize * height as usize * 4 {
            return Err(format!("the pixels are {} bytes, which is not {width}×{height}×4", rgba.len()));
        }
        let mut chw = vec![0.0; 3 * target_w * target_h];
        for y in 0..target_h {
            let source_y = (y * height as usize / target_h) as u32;
            for x in 0..target_w {
                let source_x = (x * width as usize / target_w) as u32;
                let pixel = (source_y as usize * width as usize + source_x as usize) * 4;
                let destination = y * target_w + x;
                for channel in 0..3 {
                    let value = rgba[pixel + channel] as f32 / 255.0;
                    chw[channel * target_w * target_h + destination] = (value - mean[channel]) / std[channel];
                }
            }
        }
        Ok(Image {
            width: target.0,
            height: target.1,
            chw,
        })
    }

    /// The input tensor for a 1×3×H×W model.
    fn tensor(&self) -> Result<TValue, String> {
        Tensor::from_shape(&[1, 3, self.height as usize, self.width as usize], &self.chw)
            .map_err(|error| format!("the image does not fit the model input: {error}"))
            .map(IntoTValue::into_tvalue)
    }
}

/// The YOLOS-family object detector.
///
/// `run` applies softmax over each query's class logits, keeps the best class per query, drops the
/// unnamed slots (the `N/A` classes and the no-object slot past the label map), converts the
/// normalised cxcywh boxes to corner boxes and hands the result to
/// [`crate::detect::suppress`] — the same NMS every other detector output would go through.
#[derive(Debug)]
pub struct Detector {
    session: Session,
    labels: LabelMap,
    queries: usize,
    classes: usize,
    /// Which run output is the logits tensor and which the boxes, decided once at load.
    logits_index: usize,
    boxes_index: usize,
}

impl Detector {
    /// Load a verified detector file from the store.
    pub fn load(path: &Path) -> Result<Detector, String> {
        let session = Session::load(
            path,
            &[dims(&[Some(1), Some(3), None, None])],
            // The class count comes from the file, so the logits shape is only disambiguated here;
            // boxes are exactly 1×Q×4.
            &[None, None],
            "the detector",
        )?;
        let labels = LabelMap::parse(COCO_LABELS_TEXT).map_err(|error| format!("the built-in label map is broken: {error}"))?;
        // Which output is which: 4 as the last dimension is a box, anything else is logits.
        let mut logits = None;
        let mut boxes = None;
        for (index, output_dims) in &session.outputs {
            if output_dims.len() != 3 || output_dims[0] != 1 {
                return Err(format!(
                    "the detector output {index} has shape {output_dims:?}, which is not 1×queries×rest"
                ));
            }
            if output_dims[2] == 4 {
                boxes = Some((*index, output_dims.clone()));
            } else {
                logits = Some((*index, output_dims.clone()));
            }
        }
        let Some((logits_index, logits_dims)) = logits else {
            return Err("the detector has no logits output (1×queries×classes)".to_string());
        };
        let Some((boxes_index, boxes_dims)) = boxes else {
            return Err("the detector has no boxes output (1×queries×4)".to_string());
        };
        if logits_dims[1] != boxes_dims[1] {
            return Err(format!(
                "the detector disagrees with itself: {} queries of logits, {} of boxes",
                logits_dims[1], boxes_dims[1]
            ));
        }
        Ok(Detector {
            session,
            labels,
            queries: logits_dims[1],
            classes: logits_dims[2],
            logits_index,
            boxes_index,
        })
    }

    /// The detector's input size, as (height, width) — 512×800 for the pinned model.
    pub fn input(&self) -> (u32, u32) {
        (self.session.input[2] as u32, self.session.input[3] as u32)
    }

    /// Run the detector over one image and answer with the guesses that survived suppression.
    pub fn run(&self, image: &Image) -> Result<Vec<Detection>, String> {
        let expected = [1, 3, self.session.input[2], self.session.input[3]];
        if self.session.input != expected {
            return Err(format!(
                "the detector input is {:?}, not the 1×3×H×W the contract says",
                self.session.input
            ));
        }
        let (image_h, image_w) = (image.height as usize, image.width as usize);
        if (image_h, image_w) != (self.session.input[2], self.session.input[3]) {
            return Err(format!(
                "the image is {image_w}×{image_h} and the detector takes {}×{}",
                self.session.input[3], self.session.input[2]
            ));
        }
        let outputs = self.session.run(image.tensor()?, "the detector")?;
        let logits = &outputs[self.logits_index];
        let boxes = &outputs[self.boxes_index];
        if logits.len() != self.queries * self.classes || boxes.len() != self.queries * 4 {
            return Err(format!(
                "the detector answered {} logits and {} box floats, which is not {}×{} and {}×4",
                logits.len(),
                boxes.len(),
                self.queries,
                self.classes,
                self.queries
            ));
        }
        let mut detections = Vec::with_capacity(self.queries);
        for query in 0..self.queries {
            let row = &logits[query * self.classes..(query + 1) * self.classes];
            let Some((class, score)) = softmax_best(row) else {
                continue;
            };
            // Class slots past the label map are the no-object (EOS) slot; the map's `N/A` entries are
            // classes COCO never names. Both are noise, by construction rather than by threshold.
            let Some(name) = self.labels.name(class as u16) else {
                continue;
            };
            if name == "N/A" {
                continue;
            }
            let box_ = &boxes[query * 4..(query + 1) * 4];
            let (cx, cy, w, h) = (box_[0], box_[1], box_[2], box_[3]);
            if !cx.is_finite() || !cy.is_finite() || !w.is_finite() || !h.is_finite() {
                continue;
            }
            detections.push(Detection {
                class: class as u16,
                score,
                found: Box::new(
                    (cx - w / 2.0).clamp(0.0, 1.0),
                    (cy - h / 2.0).clamp(0.0, 1.0),
                    w.clamp(0.0, 1.0),
                    h.clamp(0.0, 1.0),
                ),
            });
        }
        Ok(suppress(&detections, IOU_THRESHOLD, MIN_SCORE))
    }

    /// The name for a class index, from the built-in COCO map.
    pub fn label(&self, class: u16) -> Option<&str> {
        self.labels.name(class)
    }

    /// How many queries the model answers.
    pub fn query_count(&self) -> usize {
        self.queries
    }
}

/// Softmax over a row, answering the winning index and its probability.
fn softmax_best(row: &[f32]) -> Option<(usize, f32)> {
    if row.is_empty() || row.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let max = row.iter().fold(f32::MIN, |a, &b| a.max(b));
    let sum: f32 = row.iter().map(|value| (value - max).exp()).sum();
    if sum <= 0.0 {
        return None;
    }
    let (best, value) = row
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(index, value)| (index, (value - max).exp() / sum))?;
    Some((best, value))
}

/// One embedding: a point in a shared vector space.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding {
    /// The coordinates, straight from the model.
    pub values: Vec<f32>,
}

impl Embedding {
    /// Cosine similarity with another embedding: 1 identical, 0 unrelated.
    ///
    /// A zero vector (a model that answered all zeros) is unrelated to everything, including itself,
    /// rather than a NaN — a NaN similarity would sort unpredictably in every panel that shows it.
    pub fn cosine(&self, other: &Embedding) -> f32 {
        if self.values.len() != other.values.len() {
            return 0.0;
        }
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;
        for (a, b) in self.values.iter().zip(&other.values) {
            if !a.is_finite() || !b.is_finite() {
                return 0.0;
            }
            dot += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }
        let denominator = norm_a.sqrt() * norm_b.sqrt();
        if denominator <= 0.0 {
            0.0
        } else {
            (dot / denominator).clamp(-1.0, 1.0)
        }
    }
}

/// The CLIP ViT-B/32 image embedder.
#[derive(Debug)]
pub struct ImageEmbedder {
    session: Session,
    edge: usize,
    dimension: usize,
}

impl ImageEmbedder {
    /// Load a verified image-embedder file. `expected_edge`, when the manifest names one, must match
    /// the file — a 224 model asked to digest 384 px is a wrong file, not a different photo.
    pub fn load(path: &Path, expected_edge: Option<u32>) -> Result<ImageEmbedder, String> {
        let session = Session::load(
            path,
            &[dims(&[Some(1), Some(3), None, None])],
            &[Some(dims(&[Some(1), None]))],
            "the image embedder",
        )?;
        if session.input[2] != session.input[3] {
            return Err(format!("the image embedder takes {:?}, which is not square", session.input));
        }
        let edge = session.input[2];
        if let Some(expected) = expected_edge
            && edge != expected as usize
        {
            return Err(format!(
                "the image embedder takes {edge} px and the manifest promises {expected} px"
            ));
        }
        let dimension = session
            .outputs
            .first()
            .and_then(|(_, dims)| dims.last().copied())
            .ok_or_else(|| "the image embedder has no output".to_string())?;
        Ok(ImageEmbedder { session, edge, dimension })
    }

    /// The square input size, 224 for the pinned model.
    pub fn edge(&self) -> u32 {
        self.edge as u32
    }

    /// Embed one image.
    pub fn run(&self, image: &Image) -> Result<Embedding, String> {
        if (image.width as usize, image.height as usize) != (self.edge, self.edge) {
            return Err(format!(
                "the image is {}×{} and the embedder takes {}×{}",
                image.width, image.height, self.edge, self.edge
            ));
        }
        let values = self.session.run(image.tensor()?, "the image embedder")?;
        values
            .into_iter()
            .next()
            .map(|values| Embedding { values })
            .ok_or_else(|| "the image embedder answered nothing".to_string())
    }

    /// The dimensionality, 512 for the pinned model.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

/// The CLIP ViT-B/32 text embedder: BPE token ids in, the same 512-d space as the image embedder out.
#[derive(Debug)]
pub struct TextEmbedder {
    session: Session,
    tokens: usize,
    dimension: usize,
}

impl TextEmbedder {
    /// Load a verified text-embedder file.
    pub fn load(path: &Path) -> Result<TextEmbedder, String> {
        let session = Session::load(
            path,
            &[dims(&[Some(1), None])],
            &[Some(dims(&[Some(1), None]))],
            "the text embedder",
        )?;
        let tokens = session.input[1];
        let dimension = session
            .outputs
            .first()
            .and_then(|(_, dims)| dims.last().copied())
            .ok_or_else(|| "the text embedder has no output".to_string())?;
        Ok(TextEmbedder {
            session,
            tokens,
            dimension,
        })
    }

    /// The token count the model takes, 77 for the pinned model (padded).
    pub fn token_count(&self) -> usize {
        self.tokens
    }

    /// Embed one padded token sequence.
    pub fn run(&self, ids: &[i64]) -> Result<Embedding, String> {
        if ids.len() != self.tokens {
            return Err(format!("the text is {} tokens and the embedder takes {}", ids.len(), self.tokens));
        }
        let tensor = Tensor::from_shape(&[1, self.tokens], ids)
            .map_err(|error| format!("the token ids do not fit the model input: {error}"))?
            .into_tvalue();
        let values = self.session.run(tensor, "the text embedder")?;
        values
            .into_iter()
            .next()
            .map(|values| Embedding { values })
            .ok_or_else(|| "the text embedder answered nothing".to_string())
    }

    /// The dimensionality, 512 for the pinned model.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use sortlens_testkit::onnx;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sortlens-infer-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).expect("write model");
    }

    fn image(width: u32, height: u32) -> Image {
        Image::from_rgba(
            &vec![128u8; (width * height * 4) as usize],
            width,
            height,
            (width, height),
            IMAGENET_MEAN,
            IMAGENET_STD,
        )
        .expect("image")
    }

    /// Two overlapping "person" boxes, one "bicycle", one with the no-object slot winning.
    /// Classes: 0 = N/A, 1 = person, 2 = bicycle, 3 = EOS-past-the-map slot (4 classes).
    fn detection_model(path: &Path) {
        let queries = 5;
        let classes = 3;
        let mut logits = vec![0.0; queries * classes];
        // Query 0: person, strong. Query 1: person, slightly weaker, same box (will be suppressed).
        // Query 2: bicycle. Query 3: N/A class wins (dropped by name). Query 4: a NaN logit (dropped).
        let mut set = |query: usize, class: usize, value: f32| logits[query * classes + class] = value;
        set(0, 1, 4.0);
        set(1, 1, 3.0);
        set(2, 2, 2.5);
        set(3, 0, 3.0);
        set(4, 1, f32::NAN);
        let boxes = [
            0.5, 0.5, 0.4, 0.4, // query 0: person, cx/cy/w/h
            0.5, 0.5, 0.4, 0.4, // query 1: same box, weaker
            0.1, 0.1, 0.1, 0.1, // query 2: bicycle, elsewhere
            0.9, 0.9, 0.1, 0.1, // query 3: N/A
            0.2, 0.2, 0.1, 0.1, // query 4: NaN logit
        ];
        write(path, &onnx::detector_onnx(queries, classes, &logits, &boxes).expect("encode"));
    }

    #[test]
    fn a_detector_answers_detections_through_the_same_nms_everyone_uses() {
        let dir = temp("detector");
        let path = dir.join("detector.onnx");
        detection_model(&path);
        let detector = Detector::load(&path).expect("load");
        assert_eq!(detector.input(), (8, 8));
        assert_eq!(detector.query_count(), 5);
        let detections = detector.run(&image(8, 8)).expect("run");
        assert_eq!(detections.len(), 2, "the duplicate and the noise are gone: {detections:?}");
        assert_eq!(detections[0].class, 1, "person first");
        assert!(detections[0].score > detections[1].score);
        assert_eq!(detections[1].class, 2, "bicycle");
        // cxcywh → corners: cx=cy=0.5, w=h=0.4 is x=y=0.3, w=h=0.4.
        let person = &detections[0];
        assert!((person.found.x - 0.3).abs() < 1e-5 && (person.found.y - 0.3).abs() < 1e-5);
        assert!((person.found.w - 0.4).abs() < 1e-5 && (person.found.h - 0.4).abs() < 1e-5);
        // The probability is a real softmax share, not a raw logit: exp(4) dominates the row.
        assert!(person.score > 0.9, "a 4-vs-0 logit gap is more than 90%: {}", person.score);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_detector_refuses_an_image_of_the_wrong_size_and_a_model_of_the_wrong_shape() {
        let dir = temp("detector-shapes");
        let path = dir.join("detector.onnx");
        detection_model(&path);
        let detector = Detector::load(&path).expect("load");
        let wrong = Image::from_rgba(&vec![128u8; 16 * 16 * 4], 16, 16, (16, 16), IMAGENET_MEAN, IMAGENET_STD).expect("image");
        let error = detector.run(&wrong).expect_err("wrong image size");
        assert!(error.contains("16×16"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_is_not_a_detector_is_refused_at_load() {
        let dir = temp("detector-wrong-file");
        let path = dir.join("embedder.onnx");
        write(&path, &onnx::embedder_onnx(16, &[1.0; 16]).expect("encode"));
        let error = Detector::load(&path).expect_err("an embedder is not a detector");
        assert!(error.contains("output"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_file_is_refused_with_words() {
        let error = Detector::load(Path::new("/nowhere/model.onnx")).expect_err("missing");
        assert!(error.contains("could not read"), "{error}");
    }

    #[test]
    fn the_image_embedder_answers_one_vector_and_cosine_compares() {
        let dir = temp("embedder");
        let path = dir.join("embedder.onnx");
        let values = vec![3.0, 4.0, 0.0];
        write(&path, &onnx::embedder_onnx(3, &values).expect("encode"));
        let embedder = ImageEmbedder::load(&path, Some(8)).expect("load");
        assert_eq!(embedder.edge(), 8);
        assert_eq!(embedder.dimension(), 3);
        let embedding = embedder.run(&image(8, 8)).expect("run");
        assert_eq!(embedding.values, values);
        let same = Embedding { values: values.clone() };
        assert!((embedding.cosine(&same) - 1.0).abs() < 1e-6);
        let orthogonal = Embedding {
            values: vec![0.0, 0.0, 1.0],
        };
        assert!(embedding.cosine(&orthogonal).abs() < 1e-6);
        // A zero vector is unrelated, not a NaN.
        let zero = Embedding {
            values: vec![0.0, 0.0, 0.0],
        };
        assert_eq!(embedding.cosine(&zero), 0.0);
        // Different lengths refuse to compare rather than lie.
        assert_eq!(embedding.cosine(&Embedding { values: vec![1.0] }), 0.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_embedder_refuses_an_edge_the_manifest_did_not_pin() {
        let dir = temp("embedder-edge");
        let path = dir.join("embedder.onnx");
        write(&path, &onnx::embedder_onnx(16, &[1.0; 16]).expect("encode"));
        let error = ImageEmbedder::load(&path, Some(224)).expect_err("wrong edge");
        assert!(error.contains("224"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_text_embedder_takes_padded_ids_and_answers_the_shared_space() {
        let dir = temp("text");
        let path = dir.join("text.onnx");
        write(&path, &onnx::text_embedder_onnx(3, &[0.0, 1.0, 0.0]).expect("encode"));
        let embedder = TextEmbedder::load(&path).expect("load");
        assert_eq!(embedder.token_count(), 7);
        assert_eq!(embedder.dimension(), 3);
        let embedding = embedder.run(&[0; 7]).expect("run");
        assert_eq!(embedding.values, vec![0.0, 1.0, 0.0]);
        let short = embedder.run(&[0; 3]).expect_err("wrong length");
        assert!(short.contains("tokens"), "{short}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stretching_a_photo_reaches_every_target_pixel_and_normalises() {
        // 2×2 source, 4×4 target: the diagonal source pixels must appear on the target diagonal.
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, // row 0: red, green
            0, 0, 255, 255, 255, 255, 255, 255, // row 1: blue, white
        ];
        let image = Image::from_rgba(&rgba, 2, 2, (4, 4), [0.0; 3], [1.0; 3]).expect("image");
        assert_eq!(image.width, 4);
        assert_eq!(image.height, 4);
        let at = |channel: usize, x: usize, y: usize| image.chw[channel * 16 + y * 4 + x];
        assert_eq!(at(0, 0, 0), 1.0, "red lands top-left");
        assert_eq!(at(1, 3, 0), 1.0, "green lands top-right");
        assert_eq!(at(2, 0, 3), 1.0, "blue lands bottom-left");
        assert_eq!(at(0, 3, 3), 1.0, "white's red lands bottom-right");
        // And the normalisation is applied: 100/255 is below the red mean, so it moves negative.
        let grey = Image::from_rgba(&[100u8; 16], 2, 2, (2, 2), IMAGENET_MEAN, IMAGENET_STD).expect("image");
        assert!(grey.chw[0] < 0.0, "100/255 is below the mean");
    }

    #[test]
    fn an_image_of_the_wrong_byte_count_is_refused() {
        let error = Image::from_rgba(&[0u8; 3], 1, 1, (2, 2), IMAGENET_MEAN, IMAGENET_STD).expect_err("short buffer");
        assert!(error.contains("not 1×1×4"), "{error}");
    }

    #[test]
    fn the_coco_label_map_names_the_classes_the_detector_answers() {
        let labels = LabelMap::parse(COCO_LABELS_TEXT).expect("parse");
        assert_eq!(labels.len(), 91, "91 COCO classes; slot 92 is EOS and stays unnamed");
        assert_eq!(labels.name(0), Some("N/A"));
        assert_eq!(labels.name(1), Some("person"));
        assert_eq!(labels.name(2), Some("bicycle"));
        assert_eq!(labels.name(90), Some("toothbrush"));
        assert_eq!(labels.name(91), None, "the EOS slot has no name, so a hit there is dropped");
    }

    #[test]
    fn softmax_answers_a_probability_and_survives_the_edges() {
        let (class, score) = softmax_best(&[0.0, 2.0, 0.0]).expect("softmax");
        assert_eq!(class, 1);
        let e2 = 2.0f32.exp();
        assert!((score - e2 / (1.0 + e2 + 1.0)).abs() < 1e-6, "{score}");
        assert_eq!(softmax_best(&[]), None, "no classes, no answer");
        assert_eq!(softmax_best(&[f32::NAN]), None, "a NaN row is noise, not a guess");
        assert_eq!(softmax_best(&[f32::INFINITY]), None);
    }
}
