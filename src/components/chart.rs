//! Bar chart (gpui-kit BarChart) of labeled values at a fixed height.

use gpui_kit::component::chart::BarChart;
use gpui_kit::*;

/// `bars` as (label, value) pairs; values rounded to one decimal.
pub fn bar_chart(
    id: impl Into<ElementId>,
    name: &'static str,
    bars: Vec<(String, f64)>,
    height: Pixels,
) -> Div {
    div().h(height).child(
        BarChart::new(bars)
            .id(id)
            .name(name)
            .band(|(label, _)| SharedString::from(label.clone()))
            .value(|(_, value)| (value * 10.).round() / 10.),
    )
}
