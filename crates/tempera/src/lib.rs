//! Tempera — theme-aware UI widgets for `bevy_ui`.
//!
//! Each widget is its own Bevy plugin. Add the widget plugins you want,
//! or use [`TemperaPlugin`] to add them all.
//!
//! ## Theme tokens
//!
//! Theme data is decomposed into small, independent resources
//! ([`ColorPalette`], [`Spacing`], [`Typography`], [`FontHandle`],
//! [`MenuTokens`]). Widget systems pull only the sub-resources they
//! actually read, which keeps scheduling parallel and dependencies
//! visible.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use tempera::TemperaPlugin;
//!
//! App::new()
//!     .add_plugins((DefaultPlugins, TemperaPlugin))
//!     .run();
//! ```

use bevy::prelude::*;

pub mod anim;
pub mod theme;

pub mod button;
pub mod checkbox;
pub mod command;
mod checkbox_behavior;
pub mod context_menu;
pub mod cursor;
pub mod dialog;
pub mod dropdown_menu;
pub mod kbd;
pub mod number_field;
pub mod progress;
pub mod separator;
pub mod slider;
pub mod switch;
pub mod tabs;
pub mod text_input;
pub mod toast;
pub mod toggle_group;
pub mod tooltip;

pub use theme::{
    ColorPalette, FontHandle, MenuStyle, MenuTokens, Spacing, ThemePlugin, Typography,
};

pub mod prelude {
    pub use crate::TemperaPlugin;
    pub use crate::button::{
        spawn_button, spawn_button_sized, Activate, Button, ButtonContent, ButtonSize,
        ButtonStyle, ButtonStylePlugin, ButtonVariant, IconTint, TemperaButton,
    };
    pub use crate::command::{
        spawn_command, spawn_command_with_icon, Command, CommandActivated, CommandEmpty,
        CommandGroup, CommandGroupHeading, CommandInputRow, CommandItem, CommandItemSpec,
        CommandList, CommandPlugin, CommandSection, CommandSelection, CommandStyle,
    };
    pub use crate::checkbox::{
        spawn_checkbox, CheckGlyph, Checkbox, CheckboxStyle, CheckboxStylePlugin, Checked,
        TemperaCheckbox,
    };
    pub use crate::context_menu::{
        ContextMenuPlugin, MenuItemSpec, MenuRequest, OpenContextMenu,
    };
    pub use crate::cursor::{CursorPlugin, HoverCursor};
    pub use crate::slider::{
        spawn_slider, Slider, SliderRange, SliderSize, SliderStep, SliderStyle,
        SliderStylePlugin, SliderThumb, SliderValue, ValueChange,
    };
    pub use crate::switch::{
        spawn_switch, Switch, SwitchSize, SwitchStyle, SwitchStylePlugin, SwitchThumb,
    };
    pub use crate::toggle_group::{
        spawn_toggle_group, RadioButton, RadioGroup, ToggleGroup, ToggleGroupItem,
        ToggleGroupKind, ToggleGroupStyle, ToggleGroupStylePlugin, ToggleItem,
    };
    pub use crate::dialog::{
        spawn_dialog, Dialog, DialogBackdrop, DialogCard, DialogClose, DialogConfig,
        DialogContent, DialogDismissed, DialogParts, DialogPlugin, DialogStyle,
    };
    pub use crate::dropdown_menu::{
        spawn_dropdown, DropdownMenuPlugin, DropdownStyle, DropdownTrigger,
    };
    pub use crate::kbd::{spawn_kbd, KbdPlugin, KbdStyle};
    pub use crate::tabs::{
        spawn_tabs, TabIndicator, TabTrigger, Tabs, TabsActive, TabsChanged, TabsPlugin,
        TabsStyle,
    };
    pub use crate::progress::{
        spawn_progress, Progress, ProgressFill, ProgressPlugin, ProgressStyle, ProgressValue,
    };
    pub use crate::number_field::{
        spawn_number_field, NumberField, NumberFieldKind, NumberFieldPlugin, NumberFieldRange,
        NumberFieldStep, NumberFieldStyle, NumberFieldValue,
    };
    pub use crate::separator::{
        spawn_separator, SeparatorAxis, SeparatorPlugin, SeparatorStyle,
    };
    pub use crate::toast::{
        spawn_error as spawn_toast_error, spawn as spawn_toast, Toast, ToastConfig,
        ToastDismissible, ToastDuration, ToastExternalProgress, ToastMessage, ToastNodes,
        ToastPlugin, ToastPosition, ToastShowProgress, ToastSlide, ToastSpec, ToastTitle,
        ToastVariant,
    };
    pub use crate::tooltip::{Tooltip, TooltipArrow, TooltipPlugin, TooltipPopup, TooltipPosition};
    pub use crate::text_input::{
        spawn_text_input, SubmitText, TextInput, TextInputBuffer, TextInputContents,
        TextInputFilter, TextInputHandle, TextInputMode, TextInputNode, TextInputPrompt,
        TextInputStyle, TextInputStylePlugin,
    };
    pub use crate::theme::{
        ColorPalette, FontHandle, MenuStyle, MenuTokens, Spacing, ThemePlugin, Typography,
    };
}

/// Aggregate plugin — registers theme + every widget.
pub struct TemperaPlugin;

impl Plugin for TemperaPlugin {
    fn build(&self, app: &mut App) {
        // Each widget plugin pulls ThemePlugin itself (idempotent via
        // is_plugin_added), so the aggregate just adds the widgets.
        app.add_plugins(cursor::CursorPlugin);
        app.add_plugins(context_menu::ContextMenuPlugin);
        app.add_plugins(button::ButtonStylePlugin);
        app.add_plugins(slider::SliderStylePlugin);
        app.add_plugins(checkbox::CheckboxStylePlugin);
        app.add_plugins(switch::SwitchStylePlugin);
        app.add_plugins(toggle_group::ToggleGroupStylePlugin);
        app.add_plugins(separator::SeparatorPlugin);
        app.add_plugins(progress::ProgressPlugin);
        app.add_plugins(kbd::KbdPlugin);
        app.add_plugins(tabs::TabsPlugin);
        app.add_plugins(dialog::DialogPlugin);
        app.add_plugins(dropdown_menu::DropdownMenuPlugin);
        app.add_plugins(text_input::TextInputStylePlugin);
        app.add_plugins(number_field::NumberFieldPlugin);
        app.add_plugins(tooltip::TooltipPlugin);
        app.add_plugins(toast::ToastPlugin);
        app.add_plugins(command::CommandPlugin);
    }
}
