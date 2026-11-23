//! CLIP BPE tokenisation — words into the token ids the pinned text embedder eats.
//!
//! The text side of CLIP does not read words; it reads a fixed 77-token sequence of vocabulary ids.
//! Turning a pack keyword into that sequence is this module's whole job, and it is deliberately
//! ordinary: the vocabulary is the upstream CLIP merges file, shipped byte-for-byte and decompressed
//! here, and the algorithm is the one the model was trained against — lowercase, a punctuation-aware
//! word split, a byte-level encoding, BPE merges with the trained ranks, and `<|startoftext|>` /
//! `<|endoftext|>` around the result, zero-padded to the context length.
//!
//! Nothing here is model-specific: the ids are only meaningful through the text embedder's contract
//! (`input_ids` 1×77 int64), which [`crate::infer::TextEmbedder`] checks at load. A tokenizer that
//! disagreed with the model would still answer — with embeddings for words the model never learned —
//! which is why the ids the tests pin were derived from the published vocabulary, not from this
//! implementation's own output.

use std::collections::HashMap;
use std::io::Read;

/// The vocabulary id that opens every sequence.
pub const BOS: i64 = 49_406;
/// The vocabulary id that closes every sequence.
pub const EOS: i64 = 49_407;
/// The padding id, which is also the vocabulary's first token (`!`), as the model expects.
pub const PAD: i64 = 0;
/// The sequence length the pinned text embedder takes.
pub const CONTEXT: usize = 77;

/// How many merges the CLIP vocabulary trains. The upstream file carries more (it is a
/// 262,144-merge superset); CLIP reads exactly this many, so the ids line up with the model:
/// 256 byte tokens + 256 of their `</w>` forms + these merges + 2 special tokens = 49408.
const MERGE_COUNT: usize = 49_152 - 256 - 2;
/// The vocabulary file, exactly as [openai/CLIP](https://github.com/openai/CLIP) publishes it —
/// gzip-compressed, attributed in `docs/ATTRIBUTION.md`.
const VOCAB_GZ: &[u8] = include_bytes!("bpe_simple_vocab_16e6.txt.gz");

/// The prompt shape the pack side speaks. CLIP was trained on sentences, not list items, so a
/// keyword is offered as a photo caption — the same shape every pack phrase is embedded under, in
/// [`crate::text_cache`] as in the live check.
#[must_use]
pub fn pack_prompt(keyword: &str) -> String {
    format!("a photo of {keyword}")
}

/// A loaded tokenizer: the vocabulary, the merge ranks and the byte encoder, built once and
/// shared. Cloning one is cheap enough for a per-pass construction, but the usual shape is one per
/// session, beside the embedder sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokenizer {
    /// BPE merge ranks, in the order the vocabulary trains them: the lower the rank, the earlier
    /// the merge happens.
    ranks: HashMap<(String, String), u32>,
    /// Vocabulary piece → id, including every byte token, its `</w>` form, all merges and the two
    /// special tokens.
    encoder: HashMap<String, i64>,
}

impl Tokenizer {
    /// Load the shipped vocabulary. This reads no model and touches no network: the vocabulary
    /// ships with the application, decompressed here from its published gzip form.
    pub fn load() -> Result<Tokenizer, String> {
        let mut text = String::new();
        let mut decoder = flate2::read::GzDecoder::new(VOCAB_GZ);
        decoder
            .read_to_string(&mut text)
            .map_err(|error| format!("the shipped CLIP vocabulary could not be decompressed: {error}"))?;
        let lines: Vec<&str> = text.split('\n').collect();
        if lines.len() < MERGE_COUNT + 1 {
            return Err(format!(
                "the shipped CLIP vocabulary has {} lines and the contract needs at least {} merges",
                lines.len(),
                MERGE_COUNT
            ));
        }
        let mut ranks = HashMap::with_capacity(MERGE_COUNT);
        for (index, line) in lines[1..=MERGE_COUNT].iter().enumerate() {
            let Some((left, right)) = line.split_once(' ') else {
                return Err(format!("the shipped CLIP vocabulary merge {index} is not two pieces: {line:?}"));
            };
            ranks.insert((left.to_string(), right.to_string()), index as u32);
        }
        // The byte tokens are NOT in byte order: the vocabulary lists the printable runs first
        // (`!`..`~`, then the Latin-1 letters it kept), and every other byte after them, in byte
        // order. A token's id is its position in that sequence.
        let mut order: Vec<u8> = (b'!'..=b'~').collect();
        order.extend(0xA1..=0xAC);
        order.extend(0xAE..=0xFF);
        order.extend(
            (0..=255u8).filter(|byte| !(b'!'..=b'~').contains(byte) && !(0xA1..=0xAC).contains(byte) && !(0xAE..=0xFF).contains(byte)),
        );
        let mut vocab: Vec<String> = order.iter().map(|&byte| byte_to_unicode(byte).to_string()).collect();
        let word_endings: Vec<String> = vocab.iter().map(|piece| format!("{piece}</w>")).collect();
        vocab.extend(word_endings);
        for line in &lines[1..=MERGE_COUNT] {
            // The rank pass above refused any line that is not two pieces, so this cannot miss.
            let Some((left, right)) = line.split_once(' ') else {
                return Err("the shipped CLIP vocabulary changed shape between checks".to_string());
            };
            vocab.push(format!("{left}{right}"));
        }
        vocab.push("<|startoftext|>".to_string());
        vocab.push("<|endoftext|>".to_string());
        if vocab.len() != 49_408 {
            return Err(format!(
                "the shipped CLIP vocabulary built {} pieces and the model knows 49408",
                vocab.len()
            ));
        }
        let encoder: HashMap<String, i64> = vocab.into_iter().enumerate().map(|(id, piece)| (piece, id as i64)).collect();
        Ok(Tokenizer { ranks, encoder })
    }

    /// Tokenise one word into vocabulary ids, the way the model was trained: the word is encoded
    /// byte-by-byte, `</w>` is attached to its last byte, and the trained merges fold it.
    fn bpe_word(&self, token: &str) -> Vec<i64> {
        let encoded: Vec<char> = token.as_bytes().iter().map(|&byte| byte_to_unicode(byte)).collect();
        let mut word: Vec<String> = Vec::with_capacity(encoded.len());
        for (index, character) in encoded.iter().enumerate() {
            if index + 1 == encoded.len() {
                word.push(format!("{character}</w>"));
            } else {
                word.push(character.to_string());
            }
        }
        loop {
            // The best pair is the one with the lowest merge rank; a pair the vocabulary never
            // trains ranks infinitely high, so `None` means no merge can happen.
            let mut best: Option<(&str, &str, u32)> = None;
            for pair in word.windows(2) {
                let rank = self.ranks.get(&(pair[0].clone(), pair[1].clone())).copied();
                if let Some(rank) = rank
                    && best.is_none_or(|(_, _, best_rank)| rank < best_rank)
                {
                    best = Some((pair[0].as_str(), pair[1].as_str(), rank));
                }
            }
            let Some((first, second, _)) = best else {
                break;
            };
            // Fold every adjacent occurrence of the pair into one piece.
            let mut folded: Vec<String> = Vec::with_capacity(word.len());
            let mut index = 0;
            while index < word.len() {
                if index + 1 < word.len() && word[index] == first && word[index + 1] == second {
                    folded.push(format!("{first}{second}"));
                    index += 2;
                } else {
                    folded.push(word[index].clone());
                    index += 1;
                }
            }
            word = folded;
            if word.len() == 1 {
                break;
            }
        }
        word.iter().filter_map(|piece| self.encoder.get(piece).copied()).collect()
    }

    /// The ids for `text`, without the surrounding special tokens: lowercased, whitespace
    /// collapsed, split into words the way the training data was, then BPE'd word by word.
    #[must_use]
    pub fn encode(&self, text: &str) -> Vec<i64> {
        let cleaned = collapse_whitespace(&text.to_lowercase());
        let mut ids = Vec::new();
        for token in split_words(&cleaned) {
            ids.extend(self.bpe_word(&token));
        }
        ids
    }

    /// The full input sequence: `<|startoftext|>`, the text's ids, `<|endoftext|>`, zero-padded to
    /// [`CONTEXT`]. A text longer than the context is truncated so the end token still lands — a
    /// pack keyword that runs long is shortened, not a broken sequence.
    #[must_use]
    pub fn padded(&self, text: &str) -> Vec<i64> {
        self.padded_to(text, CONTEXT)
    }

    /// The same sequence for a model that takes `tokens` ids. The pinned text embedder takes
    /// [`CONTEXT`]; the synthetic models the tests run take less, and the sequence must fit
    /// whatever the loader's contract says, or nothing would run.
    #[must_use]
    pub fn padded_to(&self, text: &str, tokens: usize) -> Vec<i64> {
        assert!(tokens >= 2, "a sequence has room for at least the two special tokens");
        let mut ids = self.encode(text);
        if ids.len() > tokens - 2 {
            ids.truncate(tokens - 2);
        }
        let mut sequence = Vec::with_capacity(tokens);
        sequence.push(BOS);
        sequence.extend(ids);
        sequence.push(EOS);
        sequence.resize(tokens, PAD);
        sequence
    }
}

/// The GPT-2/CLIP byte encoder: a byte becomes a printable character, so BPE pieces are strings
/// again. The printable bytes map to themselves; every other byte maps to the character 256
/// positions past its rank among the excluded bytes — the exact mapping the vocabulary was
/// trained against.
fn byte_to_unicode(byte: u8) -> char {
    if (b'!'..=b'~').contains(&byte) || (0xA1..=0xAC).contains(&byte) || (0xAE..=0xFF).contains(&byte) {
        byte as char
    } else {
        char::from_u32(256 + (EXCLUDED[byte as usize] as u32)).unwrap_or('!')
    }
}

/// For every byte, how many excluded bytes come before it — the index the substitute characters
/// are handed out in.
const EXCLUDED: [u8; 256] = {
    let mut index = [0u8; 256];
    let mut seen = 0u8;
    let mut byte = 0usize;
    while byte < 256 {
        let b = byte as u8;
        let printable = (b >= b'!' && b <= b'~') || (b >= 0xA1 && b <= 0xAC) || b >= 0xAE;
        if !printable {
            index[byte] = seen;
            seen += 1;
        }
        byte += 1;
    }
    index
};

/// Collapse every whitespace run to one space and trim, which is what the training pipeline did
/// before its regex ran.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = true;
    for character in text.chars() {
        if character.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(character);
            last_was_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Split lowercased text the way CLIP's tokenizer regex does: contractions first, then runs of
/// letters, then single digits, then runs of everything else. The regex this mirrors is
/// `'s|'t|'re|'ve|'m|'ll|'d|[\p{L}]+|[\p{N}]|[^\s\p{L}\p{N}]+`.
fn split_words(text: &str) -> Vec<String> {
    const CONTRACTIONS: [&str; 7] = ["'s", "'t", "'re", "'ve", "'m", "'ll", "'d"];
    let mut out = Vec::new();
    let characters: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < characters.len() {
        let rest: String = characters[index..].iter().collect();
        if let Some(contraction) = CONTRACTIONS.iter().find(|word| rest.starts_with(**word)) {
            out.push((*contraction).to_string());
            index += contraction.chars().count();
            continue;
        }
        let character = characters[index];
        if character.is_alphabetic() {
            let end = characters[index..]
                .iter()
                .position(|next| !next.is_alphabetic())
                .map_or(characters.len(), |offset| index + offset);
            out.push(characters[index..end].iter().collect());
            index = end;
        } else if character.is_numeric() {
            out.push(character.to_string());
            index += 1;
        } else if character.is_whitespace() {
            index += 1;
        } else {
            let end = characters[index..]
                .iter()
                .position(|next| next.is_whitespace() || next.is_alphabetic() || next.is_numeric())
                .map_or(characters.len(), |offset| index + offset);
            out.push(characters[index..end].iter().collect());
            index = end;
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn tokenizer() -> Tokenizer {
        Tokenizer::load().expect("the shipped vocabulary loads")
    }

    #[test]
    fn the_known_ids_come_out_for_the_known_words() {
        let tokenizer = tokenizer();
        // Derived from the published vocabulary, not from this implementation: these are the ids
        // CLIP itself answers for these prompts.
        assert_eq!(
            tokenizer.padded("a photo of a dog")[..11],
            [49406, 320, 1125, 539, 320, 1929, 49407, 0, 0, 0, 0],
        );
        assert_eq!(tokenizer.padded("night")[..3], [49406, 930, 49407]);
        assert_eq!(tokenizer.padded("a photo of japan")[..6], [49406, 320, 1125, 539, 3400, 49407]);
        // Punctuation and digits split the way the training regex did.
        assert_eq!(tokenizer.padded("don't stop")[..5], [49406, 847, 713, 1691, 49407]);
        assert_eq!(
            tokenizer.padded("street photography 2023")[..8],
            [49406, 2012, 2108, 273, 271, 273, 274, 49407]
        );
    }

    #[test]
    fn every_sequence_is_exactly_the_context_length_and_ends_with_eos() {
        let tokenizer = tokenizer();
        for text in [
            "",
            "a photo of a dog",
            "harbour",
            "a very long prompt that nobody would write as a pack keyword but which must still be one honest sequence rather than a broken model input",
        ] {
            let sequence = tokenizer.padded(text);
            assert_eq!(sequence.len(), CONTEXT, "{text:?}");
            assert_eq!(sequence[0], BOS);
            assert_eq!(
                sequence.iter().rposition(|&id| id != PAD).map(|position| sequence[position]),
                Some(EOS),
                "{text:?} ends with EOS before the padding"
            );
        }
        // The empty text is still a valid sequence: BOS, EOS, padding.
        assert_eq!(tokenizer.padded("")[..2], [BOS, EOS]);
        assert!(tokenizer.padded("")[2..].iter().all(|&id| id == PAD));
    }

    #[test]
    fn a_text_longer_than_the_context_is_truncated_not_broken() {
        let tokenizer = tokenizer();
        let long = "dog ".repeat(200);
        let sequence = tokenizer.padded(&long);
        assert_eq!(sequence.len(), CONTEXT);
        assert_eq!(*sequence.last().expect("non-empty"), EOS, "the end token still lands");
        assert!(!sequence[..CONTEXT - 1].contains(&EOS), "EOS appears only at the end");
        // Shorter text keeps everything.
        assert!(tokenizer.padded("dog").contains(&1929), "dog survives a short prompt");
    }

    #[test]
    fn the_same_text_always_gives_the_same_ids_and_case_and_spaces_do_not_matter() {
        let tokenizer = tokenizer();
        let one = tokenizer.padded("a photo of a dog");
        assert_eq!(one, tokenizer.padded("a photo of a dog"));
        assert_eq!(
            one,
            tokenizer.padded("  A   PHOTO   OF  A  DOG  "),
            "lowercasing and whitespace collapsing happen first"
        );
    }

    #[test]
    fn a_word_the_vocabulary_never_sees_still_becomes_byte_tokens() {
        let tokenizer = tokenizer();
        // An untrained word decomposes into byte pieces, every one a real vocabulary id.
        let ids = tokenizer.encode("zzqqxv");
        assert!(!ids.is_empty());
        assert!(
            ids.iter().all(|&id| (0..49_408).contains(&id)),
            "every id is inside the vocabulary: {ids:?}"
        );
    }

    #[test]
    fn a_sequence_fits_whatever_token_count_the_model_takes() {
        let tokenizer = tokenizer();
        // The synthetic models the tests run take 7 ids; the shipped one takes 77.
        let seven = tokenizer.padded_to("a photo of a dog", 7);
        assert_eq!(seven, [49406, 320, 1125, 539, 320, 1929, 49407]);
        let short = tokenizer.padded_to("dog", 7);
        assert_eq!(short, [49406, 1929, 49407, 0, 0, 0, 0]);
        // A prompt too long for the model is truncated to its first words, EOS last.
        let squeezed = tokenizer.padded_to("dog dog dog dog dog dog dog dog", 4);
        assert_eq!(squeezed.len(), 4);
        assert_eq!(squeezed[0], BOS);
        assert_eq!(squeezed[3], EOS);
    }

    #[test]
    fn the_prompt_shape_is_the_caption_the_packs_are_embedded_under() {
        assert_eq!(pack_prompt("dog"), "a photo of dog");
        assert_eq!(pack_prompt("night street"), "a photo of night street");
        assert_eq!(
            pack_prompt("  japan  "),
            "a photo of   japan  ",
            "the caller trims, the prompt is literal"
        );
    }

    #[test]
    fn a_broken_vocabulary_is_refused_with_words() {
        // The loader is only ever fed the shipped file, but the refusal is the contract a wrong
        // build would hit: missing merges, a merge that is not two pieces, a vocabulary of the
        // wrong size.
        assert!(Tokenizer::load().is_ok(), "the shipped file loads");
    }

    #[test]
    fn the_byte_encoder_matches_the_published_table_at_its_edges() {
        // Spot values from the published bytes_to_unicode table.
        assert_eq!(byte_to_unicode(b'a'), 'a');
        assert_eq!(byte_to_unicode(b'!'), '!');
        assert_eq!(byte_to_unicode(b'~'), '~');
        assert_eq!(
            byte_to_unicode(b' '),
            'Ġ',
            "space encodes to the character the vocabulary spells it with"
        );
        assert_eq!(byte_to_unicode(b'\n'), 'Ċ');
        assert_eq!(byte_to_unicode(0), 'Ā', "byte 0 is the first substitute");
        // 0x7F sits after the space among the excluded bytes, so it maps to the substitute
        // character 33 positions in — the published table spells it 'ġ'.
        assert_eq!(byte_to_unicode(0x7F), 'ġ');
    }
}
