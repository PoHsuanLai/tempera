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
pub mod list_row;
pub mod number_field;
pub mod progress;
pub mod select;
pub mod separator;
pub mod setting_row;
pub mod slider;
pub mod switch;
pub mod tabs;
pub mod text_input;
pub mod toast;
pub mod toggle_group;
pub mod tooltip;
pub mod tree_row;

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
    pub use crate::tree_row::{
        spawn_tree_row, ChevronState, TreeRow, TreeRowChevron, TreeRowExpanded, TreeRowHeader,
        TreeRowLabel, TreeRowPlugin, TreeRowSpec, TreeRowStyle, TreeRowSuffix, TreeRowTokens,
    };
    pub use crate::dropdown_menu::{
        spawn_dropdown, DropdownMenuPlugin, DropdownStyle, DropdownTrigger,
    };
    pub use crate::kbd::{spawn_kbd, KbdPlugin, KbdStyle};
    pub use crate::list_row::{
        spawn_list_row, ListRow, ListRowBadge, ListRowId, ListRowLead, ListRowMeta, ListRowParts,
        ListRowPlugin, ListRowSpec, ListRowStyle, ListRowSubtitle, ListRowTitle, ListRowTokens,
        ListRowTrail,
    };
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
    // NB: `select::ValueChange` is intentionally not re-exported here — it would
    // clash with `slider::ValueChange`. Import it via `tempera::select::ValueChange`.
    pub use crate::select::{
        spawn_select, Select, SelectOption, SelectOptions, SelectPlugin, SelectStyle, SelectValue,
    };
    pub use crate::separator::{
        spawn_separator, SeparatorAxis, SeparatorPlugin, SeparatorStyle,
    };
    pub use crate::setting_row::{
        spawn_section_header, spawn_setting_row, SettingRow, SettingRowControl,
        SettingRowDescription, SettingRowLabel, SettingRowPlugin, SettingRowSpec, SettingRowStyle,
        SettingRowTokens, SettingSection,
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
        // is_plugin_added). Bevy panics on duplicate plugin add, so we
        // guard each sub-plugin too — downstream apps may have already
        // cherry-picked some of these (or another tempera-consuming
        // dependency may have done so transitively).
        add_once::<cursor::CursorPlugin>(app, || cursor::CursorPlugin);
        add_once::<context_menu::ContextMenuPlugin>(app, || context_menu::ContextMenuPlugin);
        add_once::<button::ButtonStylePlugin>(app, || button::ButtonStylePlugin);
        add_once::<slider::SliderStylePlugin>(app, || slider::SliderStylePlugin);
        add_once::<checkbox::CheckboxStylePlugin>(app, || checkbox::CheckboxStylePlugin);
        add_once::<switch::SwitchStylePlugin>(app, || switch::SwitchStylePlugin);
        add_once::<toggle_group::ToggleGroupStylePlugin>(
            app,
            || toggle_group::ToggleGroupStylePlugin,
        );
        add_once::<separator::SeparatorPlugin>(app, || separator::SeparatorPlugin);
        add_once::<progress::ProgressPlugin>(app, || progress::ProgressPlugin);
        add_once::<kbd::KbdPlugin>(app, || kbd::KbdPlugin);
        add_once::<tabs::TabsPlugin>(app, || tabs::TabsPlugin);
        add_once::<dialog::DialogPlugin>(app, || dialog::DialogPlugin);
        add_once::<dropdown_menu::DropdownMenuPlugin>(app, || dropdown_menu::DropdownMenuPlugin);
        add_once::<text_input::TextInputStylePlugin>(app, || text_input::TextInputStylePlugin);
        add_once::<number_field::NumberFieldPlugin>(app, || number_field::NumberFieldPlugin);
        add_once::<tooltip::TooltipPlugin>(app, || tooltip::TooltipPlugin);
        add_once::<toast::ToastPlugin>(app, || toast::ToastPlugin);
        add_once::<command::CommandPlugin>(app, || command::CommandPlugin);
        add_once::<tree_row::TreeRowPlugin>(app, || tree_row::TreeRowPlugin);
        add_once::<list_row::ListRowPlugin>(app, || list_row::ListRowPlugin);
        add_once::<setting_row::SettingRowPlugin>(app, || setting_row::SettingRowPlugin);
    }
}

/// `app.add_plugins(p)` is fatal on duplicates. This helper skips the
/// add when the same plugin type has already been registered.
fn add_once<P: Plugin>(app: &mut App, ctor: impl FnOnce() -> P) {
    if !app.is_plugin_added::<P>() {
        app.add_plugins(ctor());
    }
}
