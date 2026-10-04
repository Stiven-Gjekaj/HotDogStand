//! The actions of the glass frame: move, resize, minimize, and maximize a
//! window that has no frame from the system.

use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::window::ResizeDirection;

use crate::ui::ResizeEdge;

/// Asks the system to resize the window from an edge, as if the person
/// dragged the edge of a system frame.
pub fn start_resize(window: &slint::Window, edge: ResizeEdge) {
    let direction = match edge {
        ResizeEdge::North => ResizeDirection::North,
        ResizeEdge::South => ResizeDirection::South,
        ResizeEdge::East => ResizeDirection::East,
        ResizeEdge::West => ResizeDirection::West,
        ResizeEdge::NorthEast => ResizeDirection::NorthEast,
        ResizeEdge::NorthWest => ResizeDirection::NorthWest,
        ResizeEdge::SouthEast => ResizeDirection::SouthEast,
        ResizeEdge::SouthWest => ResizeDirection::SouthWest,
    };
    // A system that cannot do this leaves the window as it is.
    let _ = window.with_winit_window(|w| w.drag_resize_window(direction));
}

/// Maximizes the window, or restores it, and gives the new state.
pub fn toggle_maximize(window: &slint::Window) -> bool {
    let maximized = !window.is_maximized();
    window.set_maximized(maximized);
    maximized
}

/// Brings a window that is open to the front.
pub fn raise(window: &slint::Window) {
    window.set_minimized(false);
    let _ = window.with_winit_window(|w| w.focus_window());
}
