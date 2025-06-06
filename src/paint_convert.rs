use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, Read},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use bitvec::prelude::*;
use tracing_subscriber::field::debug;
use zip::ZipArchive;

use crate::model_orca::OrcaModel;

// #[cfg(feature = "nope")]
pub mod model_config {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct ModelConfig {
        #[serde(rename = "object", default)]
        pub objects: Vec<ModelObject>,
    }

    // impl ModelConfig {
    //     pub fn read_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
    //         let file = std::fs::File::open(path)?;
    //         let reader = BufReader::new(file);
    //         let mut de = quick_xml::de::Deserializer::from_reader(reader);
    //         let config: ModelConfig = serde::Deserialize::deserialize(&mut de)?;
    //         Ok(config)
    //     }
    // }

    #[derive(Debug, Deserialize)]
    pub struct ModelObject {
        #[serde(rename = "@id")]
        pub id: u32,

        #[serde(rename = "metadata", default)]
        // #[serde(deserialize_with = "de_metadata")]
        pub metadata: Vec<Metadata>,

        #[serde(rename = "part", default)]
        pub parts: Vec<Part>,
    }

    // fn de_metadata<'de, D>(d: D) -> Result<Vec<Metadata>, D::Error>
    // where
    //     D: serde::Deserializer<'de>,
    // {
    //     unimplemented!()
    // }

    #[derive(Debug, Deserialize)]
    pub struct Metadata {
        #[serde(rename = "@key", default)]
        pub key: String,

        #[serde(rename = "@value", default)]
        pub value: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Part {
        #[serde(rename = "@id")]
        pub id: String,

        #[serde(rename = "@subtype")]
        pub subtype: String,

        #[serde(rename = "metadata", default)]
        pub metadata: Vec<Metadata>,

        #[serde(rename = "mesh_stat")]
        pub mesh_stat: MeshStat,
    }

    #[derive(Debug, Deserialize)]
    pub struct MeshStat {
        #[serde(rename = "@edges_fixed")]
        pub edges_fixed: String,

        #[serde(rename = "@degenerate_facets")]
        pub degenerate_facets: String,

        #[serde(rename = "@facets_removed")]
        pub facets_removed: String,

        #[serde(rename = "@facets_reversed")]
        pub facets_reversed: String,

        #[serde(rename = "@backwards_edges")]
        pub backwards_edges: String,
    }

    impl ModelObject {
        /// Returns the name of the object from its metadata, if available
        pub fn get_name(&self) -> Option<&str> {
            self.metadata
                .iter()
                .find(|meta| meta.key == "name")
                .map(|meta| meta.value.as_str())
        }
    }

    impl Part {
        /// Returns the name of the part from its metadata, if available
        pub fn get_name(&self) -> Option<&str> {
            self.metadata
                .iter()
                .find(|meta| meta.key == "name")
                .map(|meta| meta.value.as_str())
        }
    }
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct PaintConvertInfo {
    pub colors: Vec<(u8, u8, u8)>,
    pub objects: Vec<PaintConvertObject>,
    #[serde(skip)]
    pub model_settings: edit_xml::Document,
}

#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
pub struct PaintConvertObject {
    pub name: String,
    pub id: u32,
}

impl PaintConvertInfo {
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let file = std::fs::read(path)?;
        let mut reader = std::io::Cursor::new(&file);

        let mut zip = ZipArchive::new(reader)?;

        let info_path = "Metadata/project_settings.config";

        let mut info_file = zip.by_name(info_path)?;

        let mut info: serde_json::Value = serde_json::from_reader(info_file)?;

        let x = &info["filament_colour"]
            .as_array()
            .context("filament_color not found")?;

        // convert from #rrggbb to (r, g, b)
        let colors = x
            .iter()
            .map(|c| {
                let color = c.as_str().context("color not a string")?;
                let r = u8::from_str_radix(&color[1..3], 16)?;
                let g = u8::from_str_radix(&color[3..5], 16)?;
                let b = u8::from_str_radix(&color[5..7], 16)?;
                Ok((r, g, b))
            })
            .collect::<Result<Vec<(u8, u8, u8)>>>()?;

        debug!("Loaded colors: {:?}", colors);

        let mut objects = Vec::new();

        let model_settings = {
            let model_path = "Metadata/model_settings.config";
            let mut model_file = zip.by_name(model_path)?;

            let mut s = String::new();
            model_file.read_to_string(&mut s)?;

            let model_settings = edit_xml::Document::parse_str(&s).unwrap();

            let mut de = quick_xml::de::Deserializer::from_str(&s);
            let config: model_config::ModelConfig = serde::Deserialize::deserialize(&mut de)?;

            // Extract object names
            for object in config.objects.iter() {
                if let Some(name) = object.get_name() {
                    objects.push(PaintConvertObject {
                        name: name.to_string(),
                        id: object.id,
                    });
                }
            }

            model_settings
        };

        Ok(Self {
            colors,
            objects,
            model_settings,
        })
    }
}

// #[cfg(feature = "nope")]
pub fn convert_model_color(
    mut model: OrcaModel,
    conversions: Vec<Option<usize>>,
    models: &HashMap<u32, bool>,
    // paint_info: &PaintConvertInfo,
    // from_extruder: usize,
    // to_extruder: usize,
) -> Result<OrcaModel> {
    // let mut comps: HashMap<u32, &Vec<crate::model::Component>> = HashMap::new();
    let mut comps = Vec::new();

    // #[cfg(feature = "nope")]
    for object in model.model.resources.object.iter() {
        debug!("convert_model_color: Object {}", object.id);
        // if models.get(&(object.id as u32)) != Some(&true) {
        //     debug!("skipping");
        //     continue;
        // }
        let Some(model_comps) = object.object.get_components() else {
            warn!("Object {} does not have components", object.id);
            continue;
        };

        comps.extend_from_slice(&model_comps);
    }

    let mut processed_models = HashSet::new();

    // #[cfg(feature = "nope")]
    for comp in comps.iter() {
        let path = comp.path.as_ref().unwrap();

        if processed_models.contains(&path) {
            continue;
        }

        debug!(
            "Converting color for component {}, path: {}",
            comp.objectid, path,
        );
        let sub_model = model
            .sub_models_mut()
            .get_mut(&path[1..])
            .context("From Sub-model not found in sub-models")?;

        for object in sub_model.model.resources.object.iter_mut() {
            debug!("Converting color for sub-object {}", object.id);
            let mesh = object
                .object
                .get_mesh_mut()
                .context("Object does not have a mesh")?;

            // debug!("Converting mesh color for object {}", object.id);
            convert_mesh_color(mesh, conversions.clone())?;
        }

        processed_models.insert(path);
    }

    for object in model.md.object.iter_mut() {
        for md in object.metadata.iter_mut() {
            if md.key.as_deref() == Some("extruder") {
                let Some(v) = &md.value else {
                    continue;
                };
                let Some(extruder) = v.parse::<usize>().ok() else {
                    continue;
                };

                if let Some(to_extruder) = conversions[extruder] {
                    debug!("Converting color for object {}", object.id);
                    // Convert the color for this object
                    // convert_mesh_color(object, conversions.clone())?;
                    md.value = Some(to_extruder.to_string());
                } else {
                    debug!("No conversion for extruder {}", extruder);
                }
            }
        }
    }

    Ok(model)
}

pub fn convert_mesh_color(
    mesh: &mut crate::mesh::Mesh,
    // from_extruder: usize,
    // to_extruder: usize,
    conversions: Vec<Option<usize>>,
) -> Result<()> {
    for tri in mesh.triangles.triangle.iter_mut() {
        let Some(s) = tri.mmu_orca.as_ref() else {
            continue;
        };

        // debug!("Converting triangle color for triangle {:?}", tri);
        let s2 = convert_triangle_color(&s, &conversions);

        tri.mmu_orca = Some(s2);
        // let s2 = convert_triangle_color(&s, from_extruder as u8, to_extruder as u8);
    }
    Ok(())
}

#[cfg(feature = "nope")]
pub fn convert_triangle_color(input_str: &str, from_color: u8, to_color: u8) -> String {
    if input_str.is_empty() {
        return String::new();
    }

    let mut digits: Vec<u8> = Vec::new();
    for ch in input_str.chars() {
        // Convert hex character to decimal
        let digit = match ch {
            '0'..='9' => (ch as u8 - b'0') as u8,
            'A'..='F' => (ch as u8 - b'A' + 10) as u8,
            _ => panic!("Invalid hex character in input"),
        };
        let low_4 = digit & 0b1111;
        let high_4 = digit >> 4;
        if low_4 == 0b1100 {
            digits.push(high_4);
        }
        digits.push(low_4);
    }

    debug!("len = {}", digits.len());

    let mut out: Vec<u8> = Vec::new();

    loop {
        let Some(nibble0) = digits.pop() else {
            break;
        };

        // debug!("Nibble0: {}", nibble0);

        // let bv = &nibble0.view_bits::<Msb0>()[4..];
        let bv = &nibble0.view_bits::<Msb0>();
        debug!("Bits: {}", bv);

        let state = &nibble0.view_bits::<Msb0>()[4..6].load_be::<u8>();

        // debug!("State: {}", state);

        let split_sides = nibble0 & 0b11;

        if split_sides > 0 {
            unimplemented!()
        } else {
            // state = 0, 1, 2: color number
            // state = 3: extra 4 bits used

            let color = match state {
                0 | 1 | 2 => *state,
                3 => {
                    // additional bits
                    let Some(nibble1) = digits.pop() else {
                        panic!("expected additional 4 bits");
                    };
                    nibble1 + 3
                }
                _ => panic!("Invalid state"),
            };

            // debug!("color = {}", color);

            let color = if color == from_color { to_color } else { color };

            let new_color = encode_color(color);

            out.extend_from_slice(&new_color);

            //
        }
    }

    // let bitstream = out
    //     .iter()
    //     .flat_map(|&x| x.view_bits::<Msb0>())
    //     .collect::<BitVec<Msb0, u8>>();

    // debug!("bitstream: {:?}", bitstream);

    debug!("out: {:?}", out);

    let chars = out.view_bits::<Msb0>().chunks(16);

    let mut result = String::with_capacity(out.len());

    // let x = 0b

    for c in chars {
        let c = c.load_be::<u16>();
        debug!("c: {:016b}", c);
        debug!("c: {:?}", c);
        // convert C to a hex digit
        let ch = if c < 10 {
            (b'0' + c as u8) as char
        } else {
            (b'A' + (c - 10) as u8) as char
        };
        result.push(ch);
    }

    // while let Some(nibble) = nibbles.next() {
    //     // check if there are 4 bits or 8 bits
    //     let digit = nibble.load_be::<u8>();
    //     // debug!("digit: {:04b}", digit);
    //     if digit == 0b1100 {
    //         let Some(next) = nibbles.next() else {
    //             panic!("expected additional 4 bits");
    //         };
    //         unimplemented!()
    //     } else {
    //         let ch = if digit < 10 {
    //             (b'0' + digit as u8) as char
    //         } else {
    //             (b'A' + (digit - 10) as u8) as char
    //         };
    //         result.push(ch);
    //     }
    // }

    // let mut result = String::with_capacity(out.len());
    // for digit in out.iter() {
    //     let ch = if *digit < 10 {
    //         (b'0' + *digit as u8) as char
    //     } else {
    //         (b'A' + (*digit - 10) as u8) as char
    //     };
    //     result.push(ch);
    // }

    unimplemented!()
    // String::new()
    // result
}

fn encode_color(color: u8) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    if color < 3 {
        out.push(color << 2);
    } else {
        out.push(0b1100);
        out.push(color - 3);
    }
    out
}

/// Converts triangles from one color to another in a PrusaSlicer triangle painting string
///
/// # Arguments
/// * `input_str` - The hexadecimal string representing triangle painting
/// * `from_color` - Source color index (as TriangleStateType number)
/// * `to_color` - Target color index (as TriangleStateType number)
///
/// # Returns
/// A new string with the converted triangle data
#[cfg(feature = "nope")]
pub fn convert_triangle_color(input_str: &str, from_color: u8, to_color: u8) -> String {
    // Return empty string if input is empty
    if input_str.is_empty() {
        return String::new();
    }

    // Parse the hexadecimal input string
    let mut digits = Vec::new();
    for ch in input_str.chars() {
        // Convert hex character to decimal
        let digit = match ch {
            '0'..='9' => (ch as u8 - b'0') as u32,
            'A'..='F' => (ch as u8 - b'A' + 10) as u32,
            _ => panic!("Invalid hex character in input"),
        };
        digits.push(digit);
    }

    // Process each nibble (4-bit value) in the data
    let mut nibble_idx = 0;
    while nibble_idx < digits.len() {
        let split_sides = digits[nibble_idx] & 0b11;

        // Leaf triangle case (no splits)
        if split_sides == 0 {
            let xx = (digits[nibble_idx] >> 2) & 0b11;

            if xx == 3 && nibble_idx + 1 < digits.len() {
                // Extended color case: 0b1100 followed by actual color value
                let extended_color = digits[nibble_idx + 1] + 3;

                if extended_color == from_color as u32 {
                    // Replace with target color
                    if to_color < 3 {
                        // Target is a simple color, need to restructure
                        digits[nibble_idx] = (to_color as u32) << 2; // Replace xx with to_color
                    } else {
                        // Target is another extended color
                        digits[nibble_idx + 1] = to_color as u32 - 3;
                    }
                }
                nibble_idx += 2; // Move past both nibbles
            } else {
                // Simple color case
                if xx == from_color as u32 {
                    if to_color < 3 {
                        // Replace with simple color
                        digits[nibble_idx] = (to_color as u32) << 2;
                    } else {
                        // Need to extend the color representation
                        digits[nibble_idx] = 0b1100; // 0b11 << 2
                                                     // Insert the extended color value
                        digits.insert(nibble_idx + 1, to_color as u32 - 3);
                    }
                }
                nibble_idx += 1;
            }
        } else {
            // This is a non-leaf node with split information
            nibble_idx += 1; // Skip special_side info

            // Skip all children (which will be processed separately in recursion)
            // No need to modify split triangle information
        }
    }

    // Convert back to hexadecimal string
    let mut result = String::with_capacity(digits.len());
    for digit in digits.iter() {
        let ch = if *digit < 10 {
            (b'0' + *digit as u8) as char
        } else {
            (b'A' + (*digit - 10) as u8) as char
        };
        result.push(ch);
    }

    result
}

/// Converts triangles from one color to another in a PrusaSlicer triangle painting string
///
/// # Arguments
/// * `input_str` - The hexadecimal string representing triangle painting
/// * `from_color` - Source color index (as TriangleStateType number)
/// * `to_color` - Target color index (as TriangleStateType number)
///
/// # Returns
/// A new string with the converted triangle data
#[cfg(feature = "nope")]
pub fn convert_triangle_color(input_str: &str, from_color: usize, to_color: usize) -> String {
    // Return empty string if input is empty
    if input_str.is_empty() {
        return String::new();
    }

    let from_color = from_color as u8;
    let to_color = to_color as u8;

    // Convert hex string to a bitstream like PrusaSlicer does
    let mut bitstream = Vec::new();
    for ch in input_str.chars().rev() {
        // Convert hex character to decimal
        let dec = match ch {
            '0'..='9' => (ch as u8 - b'0') as u32,
            'A'..='F' => (ch as u8 - b'A' + 10) as u32,
            _ => panic!("Invalid hex character in input"),
        };

        // Convert to 4 bits and append to bitstream
        for i in 0..4 {
            bitstream.push((dec & (1 << i)) != 0);
        }
    }

    // Process the bitstream and replace colors
    let mut offset = 0;
    while offset < bitstream.len() {
        // Get split_sides value (first 2 bits)
        let split_sides = bits_to_int(&bitstream, offset, 2);

        if split_sides == 0 {
            // This is a leaf triangle, check its state
            // Get xx value (next 2 bits)
            let xx = bits_to_int(&bitstream, offset + 2, 2);

            if xx == 3 && offset + 8 <= bitstream.len() {
                // Complex case: extended color stored in next 4 bits after 0b1100
                let state_offset = offset + 4;
                let current_state = bits_to_int(&bitstream, state_offset, 4) + 3;

                if current_state == from_color as u32 {
                    // Replace with target color by updating those 4 bits
                    let new_state = to_color as u32 - 3;
                    for i in 0..4 {
                        bitstream[state_offset + i] = (new_state & (1 << i)) != 0;
                    }
                }
                offset += 8; // Move past this extended color entry
            } else {
                // Simple case: color stored in xx (0, 1, or 2)
                if xx == from_color as u32 {
                    // Replace with target color
                    for i in 0..2 {
                        bitstream[offset + 2 + i] = (to_color as u32 & (1 << i)) != 0;
                    }
                }
                offset += 4; // Move past this simple color entry
            }
        } else {
            // This is a split node, skip the special_side bits
            offset += 4;
            // Skip all its children which will be processed separately
        }
    }

    // Convert bitstream back to hex string
    let mut result = String::new();
    let mut i = 0;
    while i + 3 < bitstream.len() {
        let mut digit = 0;
        for j in 0..4 {
            if bitstream[i + j] {
                digit |= 1 << j;
            }
        }

        let ch = if digit < 10 {
            (b'0' + digit as u8) as char
        } else {
            (b'A' + (digit - 10) as u8) as char
        };

        result.insert(0, ch);
        i += 4;
    }

    result
}

/// Helper function to convert a slice of bits to an integer
fn bits_to_int(bits: &[bool], offset: usize, num_bits: usize) -> u32 {
    let mut result = 0;
    for i in 0..num_bits {
        if bits[offset + i] {
            result |= 1 << i;
        }
    }
    result
}

/// Converts triangles in a PrusaSlicer painting from one extruder color to another
///
/// # Arguments
/// * `hex_string` - The hexadecimal string representation of a painted triangle
/// * `from_extruder` - The extruder number to convert from (1-based, e.g., 1 means Extruder1)
/// * `to_extruder` - The extruder number to convert to (1-based, e.g., 2 means Extruder2)
///
/// # Returns
/// A new hex string with the converted triangle data
// #[cfg(feature = "nope")]
pub fn convert_triangle_color(hex_string: &str, conversions: &[Option<usize>]) -> String {
    // If the string is empty, there's no painting data
    if hex_string.is_empty() {
        return String::new();
    }

    // Convert the string to a bitstream
    let mut bitstream = Vec::new();
    // Process the string in reverse order
    for ch in hex_string.chars().rev() {
        // Convert hex character to decimal
        let dec = match ch {
            '0'..='9' => (ch as u8 - b'0') as u32,
            'A'..='F' => 10 + (ch as u8 - b'A') as u32,
            _ => panic!("Invalid hex character in triangle data"),
        };

        // Convert decimal to 4 bits and append
        for i in 0..4 {
            bitstream.push((dec & (1 << i)) > 0);
        }
    }

    let mut idx = 0;
    let mut modified = false;
    let mut new_bitstream = Vec::new();

    // Process each nibble
    while idx + 3 < bitstream.len() {
        // Read first nibble
        let mut first_nibble = 0;
        for bit_idx in 0..4 {
            if bitstream[idx + bit_idx] {
                first_nibble |= 1 << bit_idx;
            }
        }

        // Check if this is a leaf triangle (not split)
        let is_split = (first_nibble & 0b11) != 0;

        if !is_split {
            // This is a leaf triangle - determine its state
            let current_state = if (first_nibble & 0b1100) == 0b1100 {
                // Extended state format (>=3)
                if idx + 7 < bitstream.len() {
                    // Read second nibble for extended state
                    let mut second_nibble = 0;
                    for bit_idx in 0..4 {
                        if bitstream[idx + 4 + bit_idx] {
                            second_nibble |= 1 << bit_idx;
                        }
                    }
                    second_nibble + 3
                } else {
                    // Malformed data
                    idx += 4;
                    continue;
                }
            } else {
                // Simple state format (0-2)
                first_nibble >> 2
            };

            let from_extruder = current_state as usize;
            // #[cfg(feature = "nope")]
            if let Some(to_extruder) = conversions[from_extruder] {
                modified = true;

                // Add appropriate nibbles to new_bitstream based on target state
                if to_extruder < 3 {
                    // Target is simple format (0-2)
                    let new_nibble = (to_extruder as u32) << 2;
                    for bit_idx in 0..4 {
                        new_bitstream.push((new_nibble & (1 << bit_idx)) > 0);
                    }

                    if (first_nibble & 0b1100) == 0b1100 {
                        // Skip the second nibble of extended format
                        idx += 8;
                    } else {
                        idx += 4;
                    }
                } else {
                    // Target is extended format (>=3)
                    // First nibble: 1100 (extended format indicator)
                    new_bitstream.push(false); // bit 0
                    new_bitstream.push(false); // bit 1
                    new_bitstream.push(true); // bit 2
                    new_bitstream.push(true); // bit 3

                    // Second nibble: value - 3
                    let extended_value = to_extruder as u32 - 3;
                    for bit_idx in 0..4 {
                        new_bitstream.push((extended_value & (1 << bit_idx)) > 0);
                    }

                    if (first_nibble & 0b1100) == 0b1100 {
                        // Skip both nibbles of the original extended format
                        idx += 8;
                    } else {
                        idx += 4;
                    }
                }
                continue;
            }

            // let from_extruder = 2;
            // let to_extruder = 4;

            // If this matches our from_extruder, convert it
            #[cfg(feature = "nope")]
            if current_state == from_extruder as u32 {
                modified = true;

                // Add appropriate nibbles to new_bitstream based on target state
                if to_extruder < 3 {
                    // Target is simple format (0-2)
                    let new_nibble = (to_extruder as u32) << 2;
                    for bit_idx in 0..4 {
                        new_bitstream.push((new_nibble & (1 << bit_idx)) > 0);
                    }

                    if (first_nibble & 0b1100) == 0b1100 {
                        // Skip the second nibble of extended format
                        idx += 8;
                    } else {
                        idx += 4;
                    }
                } else {
                    // Target is extended format (>=3)
                    // First nibble: 1100 (extended format indicator)
                    new_bitstream.push(false); // bit 0
                    new_bitstream.push(false); // bit 1
                    new_bitstream.push(true); // bit 2
                    new_bitstream.push(true); // bit 3

                    // Second nibble: value - 3
                    let extended_value = to_extruder as u32 - 3;
                    for bit_idx in 0..4 {
                        new_bitstream.push((extended_value & (1 << bit_idx)) > 0);
                    }

                    if (first_nibble & 0b1100) == 0b1100 {
                        // Skip both nibbles of the original extended format
                        idx += 8;
                    } else {
                        idx += 4;
                    }
                }
                continue;
            }
        }

        // Copy over the current nibble unchanged
        for i in 0..4 {
            new_bitstream.push(bitstream[idx + i]);
        }
        idx += 4;

        // If this was extended format, copy the second nibble too
        if !is_split && (first_nibble & 0b1100) == 0b1100 && idx + 3 < bitstream.len() {
            for i in 0..4 {
                new_bitstream.push(bitstream[idx + i]);
            }
            idx += 4;
        }
    }

    // If no modifications were made, return the original string
    if !modified {
        return hex_string.to_string();
    }

    // Convert bitstream back to hex string
    let mut result = String::new();
    let mut i = 0;
    while i + 3 < new_bitstream.len() {
        let mut nibble = 0;
        for bit_idx in 0..4 {
            if new_bitstream[i + bit_idx] {
                nibble |= 1 << bit_idx;
            }
        }

        // Convert nibble to hex character
        let hex_char = if nibble < 10 {
            std::char::from_digit(nibble, 16).unwrap()
        } else {
            std::char::from_digit(nibble, 16)
                .unwrap()
                .to_ascii_uppercase()
        };

        // Insert at beginning (to maintain correct order)
        result.insert(0, hex_char);
        i += 4;
    }

    result
}

/// Converts triangles in a PrusaSlicer painting from one extruder color to another
///
/// # Arguments
/// * `hex_string` - The hexadecimal string representation of a painted triangle
/// * `from_extruder` - The extruder number to convert from (1-based, e.g., 1 means Extruder1)
/// * `to_extruder` - The extruder number to convert to (1-based, e.g., 2 means Extruder2)
///
/// # Returns
/// A new hex string with the converted triangle data
#[cfg(feature = "nope")]
pub fn convert_triangle_color(
    hex_string: &str,
    from_extruder: usize,
    to_extruder: usize,
) -> String {
    // If the string is empty, there's no painting data
    if hex_string.is_empty() {
        return String::new();
    }

    // Calculate the state values for the extruders
    // In PrusaSlicer, TriangleStateType::Extruder1 = 1, Extruder2 = 2, etc.
    let from_state = from_extruder; // Extruder numbers are 1-based
    let to_state = to_extruder; // Extruder numbers are 1-based

    let mut result = String::new();

    // Process each hex character in reverse (same as PrusaSlicer's set_triangle_from_string)
    for ch in hex_string.chars().rev() {
        debug!("Converting hex character: {}", ch);

        // Convert hex character to decimal
        let mut dec = match ch {
            '0'..='9' => (ch as u8 - b'0') as u32,
            'A'..='F' => 10 + (ch as u8 - b'A') as u32,
            _ => panic!("Invalid hex character in triangle data"),
        };

        debug!("Decimal value: {}", dec);

        // Convert decimal to 4 bits
        let mut bits = [
            (dec & 0b0001) > 0,
            (dec & 0b0010) > 0,
            (dec & 0b0100) > 0,
            (dec & 0b1000) > 0,
        ];

        debug!("Bits: {:?}", bits);

        // In PrusaSlicer, the bitstream contains triangle state data
        // Check all 4 bits and if they match the from_state, change to to_state
        // The bits need to be interpreted in groups that represent the state

        // This is a simplified version - the actual mapping would depend on how
        // PrusaSlicer encodes the different triangle states in the bitstream
        // For a proper implementation, we'd need to understand the exact mapping

        // A simple substitution approach (assuming each 4-bit group is a state):
        if dec == from_state as u32 {
            dec = to_state as u32;
        }

        // Convert back to hex
        let new_ch = if dec < 10 {
            std::char::from_digit(dec, 16).unwrap()
        } else {
            std::char::from_digit(dec, 16).unwrap().to_ascii_uppercase()
        };

        result.insert(0, new_ch);
    }

    result
}
