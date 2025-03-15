use std::{path::PathBuf, time::Instant};

use anyhow::{anyhow, bail, ensure, Context, Result};
use egui::{ColorImage, TextureHandle, Vec2};
use tracing::{debug, error, info, trace, warn};

use crate::{model_orca::OrcaModel, paint_convert::PaintConvertInfo};

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    pub(super) current_tab: Tab,

    pub(super) input_files_color_convert: Vec<PathBuf>,
    pub(super) color_convert_file_info: Option<(PathBuf, PaintConvertInfo)>,
    // pub(super) color_convert_from_to: Vec<String>,
    pub(super) color_convert_from_to: Vec<Option<usize>>,
    pub(super) color_convert_in_place: bool,

    pub(super) input_files_splitting: Vec<PathBuf>,
    pub(super) input_files_conversion: Vec<PathBuf>,
    pub(super) input_files_instancing: Vec<PathBuf>,
    pub(super) output_folder: Option<PathBuf>,
    #[serde(skip)]
    pub(super) processing_rx: Option<crossbeam_channel::Receiver<crate::ProcessingEvent>>,
    #[serde(skip)]
    pub(super) messages: Vec<String>,
    #[serde(skip)]
    pub(super) start_time: Option<Instant>,
    #[serde(skip)]
    pub(super) loaded_instance_file: Option<LoadedInstanceFile>,
}

#[derive(Clone)]
pub struct LoadedInstanceFile {
    pub(super) path: PathBuf,
    pub(super) orca_model: OrcaModel,
    pub(super) objects: Vec<(usize, String, bool)>,
    pub(super) from_object: Option<usize>,
    // to_objects: HashMap<usize, bool>,
    pub(super) to_objects: Vec<bool>,
    // // pub(super) preview: image::RgbImage,
    // pub(super) preview: Option<ColorImage>,
    // pub(super) preview_texture: Option<TextureHandle>,
    // pub(super) preview_changed: bool,
    // pub(super) preview_size: Vec2,
    pub(super) preview_imgs: Vec<(usize, ColorImage)>,
    pub(super) preview_texture_handles: Vec<(usize, TextureHandle)>,
    pub(super) preview_size: Vec2,
}

impl LoadedInstanceFile {
    pub fn new(
        path: PathBuf,
        orca_model: OrcaModel,
        // objects: Vec<(usize, String, bool)>,
        from_object: Option<usize>,
        // to_objects: Vec<bool>,
    ) -> Self {
        let preview_size = Vec2::new(
            orca_model.preview_size as f32,
            orca_model.preview_size as f32,
        );
        // let preview = crate::model_2d_display::model_to_image(preview_size, &orca_model).unwrap();

        let preview_imgs = orca_model
            .previews
            .iter()
            .map(|(id, img)| {
                let pixels = img.as_flat_samples();
                (
                    *id,
                    ColorImage::from_rgba_unmultiplied(
                        [preview_size.x as _, preview_size.y as _],
                        pixels.as_slice(),
                    ),
                )
            })
            .collect();

        let objects: Vec<_> = orca_model
            .get_objects()
            .iter()
            .enumerate()
            .map(|(i, ob)| {
                let name = orca_model
                    .md
                    .get_object_by_id(ob.id)
                    .unwrap()
                    .get_name()
                    .unwrap();

                let painted = *orca_model.painted.get(&ob.id).unwrap_or(&false);

                (i, name, painted)
            })
            .collect();

        let to_objects = vec![false; objects.len()];

        Self {
            path,
            orca_model,
            objects,
            from_object,
            to_objects,
            // preview: Some(preview),
            // preview: None,
            // preview_texture: None,
            // preview_changed: false,
            // preview_size,
            preview_imgs,
            preview_texture_handles: vec![],
            preview_size,
        }
    }
}

impl App {
    pub fn current_input_files(&self) -> &Vec<PathBuf> {
        match self.current_tab {
            Tab::ColorConvert => &self.input_files_color_convert,
            Tab::Conversion => &self.input_files_conversion,
            Tab::Splitting => &self.input_files_splitting,
            Tab::InstancePaint => &self.input_files_instancing,
        }
    }

    pub fn current_input_files_mut(&mut self) -> &mut Vec<PathBuf> {
        match self.current_tab {
            Tab::ColorConvert => &mut self.input_files_color_convert,
            Tab::Conversion => &mut self.input_files_conversion,
            Tab::Splitting => &mut self.input_files_splitting,
            Tab::InstancePaint => &mut self.input_files_instancing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Tab {
    ColorConvert,
    Conversion,
    Splitting,
    InstancePaint,
}

impl Default for Tab {
    fn default() -> Self {
        // Self::Splitting
        Self::InstancePaint
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Self::default()
    }
}
