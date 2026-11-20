pub mod group;
pub mod help;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WindowType {
    Group,
    Help,
    Settings,
}
