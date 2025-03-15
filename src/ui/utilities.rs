use egui::text::LayoutJob;

pub fn colored_box(color: (u8, u8, u8), text: Option<&str>) -> LayoutJob {
    let color_box = "\u{2B1B}";

    let mut job = egui::text::LayoutJob::default();

    job.append(
        color_box,
        0.0,
        egui::TextFormat {
            color: egui::Color32::from_rgb(color.0, color.1, color.2),
            ..Default::default()
        },
    );

    if let Some(text) = text {
        job.append(
            &format!(" {}", text),
            2.0,
            egui::TextFormat {
                ..Default::default()
            },
        );
    }

    job
}
