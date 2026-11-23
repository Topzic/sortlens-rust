//! Synthetic ONNX models, assembled with the same protobuf types the loader parses.
//!
//! The pinned model files are 13–89 MB and live behind the opt-in downloader, so no test can carry
//! them — and no test should: what the tests have to pin is the *plumbing* (the loader contract, the
//! post-processing, the NMS, the merge), not the weights. These builders emit tiny, valid ONNX
//! models with the pinned models' exact I/O signatures whose outputs are constants supplied by the
//! test, so a test can say "this model answers this" and see the whole pipeline answer with it.
//!
//! Real-model behaviour is not covered here; it is pinned by digest and was verified to load and run
//! under this exact tract version before the digest was recorded. What these fixtures prove is that
//! the code between the file and the suggestion panel is right.

use tract_onnx::pb::tensor_proto::DataType;
use tract_onnx::pb::tensor_proto::DataType as OnnxDatatype;
use tract_onnx::pb::tensor_shape_proto::Dimension;
use tract_onnx::pb::tensor_shape_proto::dimension::Value as DimValue;
use tract_onnx::pb::type_proto::{Tensor as TensorType, Value as TypeValue};
use tract_onnx::pb::{GraphProto, ModelProto, NodeProto, OperatorSetIdProto, TensorProto, TensorShapeProto, TypeProto, ValueInfoProto};

use prost::Message;

/// A shape, as ONNX static dimensions.
fn shape(dims: &[i64]) -> TensorShapeProto {
    TensorShapeProto {
        dim: dims
            .iter()
            .map(|&value| Dimension {
                denotation: String::new(),
                value: Some(DimValue::DimValue(value)),
            })
            .collect(),
    }
}

/// A declared tensor value with a concrete type and shape.
fn value_info(name: &str, data_type: DataType, dims: &[i64]) -> ValueInfoProto {
    ValueInfoProto {
        name: name.to_string(),
        r#type: Some(TypeProto {
            denotation: String::new(),
            value: Some(TypeValue::TensorType(TensorType {
                elem_type: data_type as i32,
                shape: Some(shape(dims)),
            })),
        }),
        doc_string: String::new(),
    }
}

/// An f32 constant tensor.
fn float_tensor(name: &str, dims: &[i64], data: &[f32]) -> TensorProto {
    TensorProto {
        dims: dims.to_vec(),
        data_type: OnnxDatatype::Float as i32,
        name: name.to_string(),
        float_data: data.to_vec(),
        ..TensorProto::default()
    }
}

/// Encode a model: one graph named `name`, the given inputs and outputs, nodes `Identity`-ing each
/// constant into its output. `Identity` of an initializer is the smallest valid ONNX graph that
/// carries a typed constant out, and it makes the output type flow through tract's inference.
fn encode(name: &str, inputs: &[ValueInfoProto], outputs: &[ValueInfoProto], tensors: &[TensorProto]) -> Result<Vec<u8>, String> {
    let nodes: Vec<NodeProto> = tensors
        .iter()
        .enumerate()
        .map(|(index, tensor)| NodeProto {
            input: vec![tensor.name.clone()],
            output: vec![outputs[index].name.clone()],
            op_type: "Identity".to_string(),
            name: format!("constant_{index}"),
            ..NodeProto::default()
        })
        .collect();
    let model = ModelProto {
        ir_version: 8,
        opset_import: vec![OperatorSetIdProto {
            domain: String::new(),
            version: 17,
        }],
        producer_name: "sortlens-testkit".to_string(),
        graph: Some(GraphProto {
            node: nodes,
            name: name.to_string(),
            initializer: tensors.to_vec(),
            input: inputs.to_vec(),
            output: outputs.to_vec(),
            ..GraphProto::default()
        }),
        ..ModelProto::default()
    };
    let mut bytes = Vec::new();
    model
        .encode(&mut bytes)
        .map_err(|error| format!("a synthetic model must encode: {error}"))?;
    Ok(bytes)
}

/// A YOLOS-shaped detector: `pixel_values` 1×3×H×W f32 in, `logits` 1×Q×C and `pred_boxes` 1×Q×4 f32
/// out, where the logits and boxes are the constants `logits` and `boxes` supplied by the test.
///
/// `logits` must be Q×C, `boxes` Q×4.
pub fn detector_onnx(queries: usize, classes: usize, logits: &[f32], boxes: &[f32]) -> Result<Vec<u8>, String> {
    if logits.len() != queries * classes {
        return Err(format!("logits must be {queries} × {classes}, got {}", logits.len()));
    }
    if boxes.len() != queries * 4 {
        return Err(format!("boxes must be {queries} × 4, got {}", boxes.len()));
    }
    let logits_tensor = float_tensor("logits_constant", &[1, queries as i64, classes as i64], logits);
    let boxes_tensor = float_tensor("boxes_constant", &[1, queries as i64, 4], boxes);
    encode(
        "synthetic_detector",
        &[value_info("pixel_values", DataType::Float, &[1, 3, 8, 8])],
        &[
            value_info("logits", DataType::Float, &[1, queries as i64, classes as i64]),
            value_info("pred_boxes", DataType::Float, &[1, queries as i64, 4]),
        ],
        &[logits_tensor, boxes_tensor],
    )
}

/// A CLIP-shaped image embedder: `pixel_values` 1×3×8×8 f32 in, `image_embeds` 1×dim f32 out.
pub fn embedder_onnx(dim: usize, values: &[f32]) -> Result<Vec<u8>, String> {
    if values.len() != dim {
        return Err(format!("values must be {dim} long, got {}", values.len()));
    }
    let tensor = float_tensor("embeds_constant", &[1, dim as i64], values);
    encode(
        "synthetic_image_embedder",
        &[value_info("pixel_values", DataType::Float, &[1, 3, 8, 8])],
        &[value_info("image_embeds", DataType::Float, &[1, dim as i64])],
        &[tensor],
    )
}

/// A CLIP-shaped text embedder: `input_ids` 1×7 int64 in, `text_embeds` 1×dim f32 out.
pub fn text_embedder_onnx(dim: usize, values: &[f32]) -> Result<Vec<u8>, String> {
    if values.len() != dim {
        return Err(format!("values must be {dim} long, got {}", values.len()));
    }
    let tensor = float_tensor("text_embeds_constant", &[1, dim as i64], values);
    encode(
        "synthetic_text_embedder",
        &[value_info("input_ids", DataType::Int64, &[1, 7])],
        &[value_info("text_embeds", DataType::Float, &[1, dim as i64])],
        &[tensor],
    )
}

/// A CLIP-shaped text embedder that answers from its input: a lookup table keyed by token id.
///
/// `table` maps vocabulary ids to embeddings (all other ids answer zeros; ids must be 0..=49407);
/// the model gathers one row per input token and sums them, so padded sequences contribute their
/// pad rows[*id as usize * dim as usize..(*id as usize + 1) * dim as usize]— put the zeros there, and give the special ids whatever the test wants them to be.
/// This is what lets a test say “the ids for *dog* embed as this vector, the ids for *cat* as
/// that one” and watch the engine answer from real tokenised text.
pub fn text_embedder_lookup_onnx(table: &[(i64, &[f32])]) -> Result<Vec<u8>, String> {
    let Some((_, first)) = table.first() else {
        return Err("a lookup table needs at least one entry".to_string());
    };
    let dim = first.len() as i64;
    if dim == 0 || table.iter().any(|(_, values)| values.len() as i64 != dim) {
        return Err("every table entry must carry the same non-zero dimension".to_string());
    }
    let vocab: usize = table.iter().map(|(id, _)| *id).max().unwrap_or(0).max(0) as usize + 1;
    if vocab > 49_408 {
        return Err(format!("the table reaches id {vocab}, past the vocabulary"));
    }
    let mut rows = vec![0.0f32; vocab * dim as usize];
    for (id, values) in table {
        rows[*id as usize * dim as usize..(*id as usize + 1) * dim as usize].copy_from_slice(values);
    }
    let table_tensor = float_tensor("text_lookup", &[vocab as i64, dim], &rows);
    // ReduceSum-13 takes its axes as an input, not an attribute; keepdims=0 folds the token axis.
    let axes = TensorProto {
        dims: vec![1],
        data_type: OnnxDatatype::Int64 as i32,
        name: "token_axis".to_string(),
        int64_data: vec![1],
        ..TensorProto::default()
    };
    let model = ModelProto {
        ir_version: 8,
        opset_import: vec![OperatorSetIdProto {
            domain: String::new(),
            version: 17,
        }],
        producer_name: "sortlens-testkit".to_string(),
        graph: Some(GraphProto {
            node: vec![
                NodeProto {
                    input: vec!["text_lookup".to_string(), "input_ids".to_string()],
                    output: vec!["gathered".to_string()],
                    op_type: "Gather".to_string(),
                    attribute: vec![tract_onnx::pb::AttributeProto {
                        name: "axis".to_string(),
                        r#type: tract_onnx::pb::attribute_proto::AttributeType::Int as i32,
                        i: 0,
                        ..tract_onnx::pb::AttributeProto::default()
                    }],
                    name: "lookup".to_string(),
                    ..NodeProto::default()
                },
                NodeProto {
                    input: vec!["gathered".to_string(), "token_axis".to_string()],
                    output: vec!["text_embeds".to_string()],
                    op_type: "ReduceSum".to_string(),
                    attribute: vec![tract_onnx::pb::AttributeProto {
                        name: "keepdims".to_string(),
                        r#type: tract_onnx::pb::attribute_proto::AttributeType::Int as i32,
                        i: 0,
                        ..tract_onnx::pb::AttributeProto::default()
                    }],
                    name: "sum".to_string(),
                    ..NodeProto::default()
                },
            ],
            name: "synthetic_text_embedder_lookup".to_string(),
            initializer: vec![table_tensor, axes],
            input: vec![value_info("input_ids", DataType::Int64, &[1, 7])],
            output: vec![value_info("text_embeds", DataType::Float, &[1, dim])],
            ..GraphProto::default()
        }),
        ..ModelProto::default()
    };
    let mut bytes = Vec::new();
    model
        .encode(&mut bytes)
        .map_err(|error| format!("a synthetic model must encode: {error}"))?;
    Ok(bytes)
}
