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

/// Design tokens — re-exported from the [`tempera_theme`] crate.
///
/// The tokens live in their own crate because they do not depend on any
/// widget: a dock or a tree that only needs a `ColorPalette` can depend on
/// `tempera-theme` alone rather than compiling this library to get one.
/// This alias keeps every existing `tempera::theme::*` path working.
pub use tempera_theme as theme;

pub mod menu_tokens;

pub mod button;
pub mod card;
pub mod checkbox;
mod checkbox_behavior;
pub mod command;
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
    Base, ColorPalette, ControlHeight, ControlSize, Density, FontHandle, Gap, Incoherent, Metrics,
    Radius, Scale, Sizing, Spacing, Step, TextScale, TextSize, ThemeConfig, ThemePlugin, Tokens,
    Typography,
};

pub mod prelude {
    pub use crate::TemperaPlugin;
    pub use crate::button::{
        Activate, Button, ButtonContent, ButtonSize, ButtonStyle, ButtonStylePlugin, ButtonVariant,
        IconTint, Selected, TemperaButton, spawn_button, spawn_button_sized,
    };
    pub use crate::card::{
        Card, CardBody, CardChevron, CardExpanded, CardHeader, CardParts, CardPlugin, CardState,
        CardStyle, CardTokens, spawn_card,
    };
    pub use crate::checkbox::{
        CheckGlyph, Checkbox, CheckboxStyle, CheckboxStylePlugin, Checked, TemperaCheckbox,
        spawn_checkbox,
    };
    pub use crate::command::{
        Command, CommandActivated, CommandEmpty, CommandGroup, CommandGroupHeading,
        CommandInputRow, CommandItem, CommandItemSpec, CommandList, CommandPlugin, CommandSection,
        CommandSelection, CommandStyle, spawn_command, spawn_command_with_icon,
    };
    pub use crate::context_menu::{ContextMenuPlugin, MenuItemSpec, MenuRequest, OpenContextMenu};
    pub use crate::cursor::{CursorPlugin, HoverCursor};
    pub use crate::dialog::{
        Dialog, DialogBackdrop, DialogCard, DialogClose, DialogConfig, DialogContent,
        DialogDismissed, DialogParts, DialogPlugin, DialogStyle, spawn_dialog,
    };
    pub use crate::dropdown_menu::{
        DropdownMenuPlugin, DropdownStyle, DropdownTrigger, spawn_dropdown,
    };
    pub use crate::kbd::{KbdPlugin, KbdStyle, spawn_kbd};
    pub use crate::list_row::{
        ListRow, ListRowBadge, ListRowId, ListRowLead, ListRowMeta, ListRowParts, ListRowPlugin,
        ListRowSpec, ListRowStyle, ListRowSubtitle, ListRowTitle, ListRowTokens, ListRowTrail,
        spawn_list_row,
    };
    pub use crate::number_field::{
        NumberField, NumberFieldKind, NumberFieldPlugin, NumberFieldRange, NumberFieldStep,
        NumberFieldStyle, NumberFieldValue, spawn_number_field,
    };
    pub use crate::progress::{
        Progress, ProgressFill, ProgressPlugin, ProgressStyle, ProgressValue, spawn_progress,
    };
    pub use crate::slider::{
        Slider, SliderRange, SliderSize, SliderStep, SliderStyle, SliderStylePlugin, SliderThumb,
        SliderValue, ValueChange, spawn_slider,
    };
    pub use crate::switch::{
        Switch, SwitchSize, SwitchStyle, SwitchStylePlugin, SwitchThumb, spawn_switch,
    };
    pub use crate::tabs::{
        TabIndicator, TabTrigger, Tabs, TabsActive, TabsChanged, TabsPlugin, TabsStyle, spawn_tabs,
    };
    pub use crate::toggle_group::{
        RadioButton, RadioGroup, ToggleGroup, ToggleGroupItem, ToggleGroupKind, ToggleGroupStyle,
        ToggleGroupStylePlugin, ToggleItem, spawn_toggle_group,
    };
    pub use crate::tree_row::{
        ChevronState, TreeRow, TreeRowChevron, TreeRowExpanded, TreeRowHeader, TreeRowLabel,
        TreeRowPlugin, TreeRowSpec, TreeRowStyle, TreeRowSuffix, TreeRowTokens, spawn_tree_row,
    };
    // NB: `select::ValueChange` is intentionally not re-exported here — it would
    // clash with `slider::ValueChange`. Import it via `tempera::select::ValueChange`.
    pub use crate::menu_tokens::{MenuStyle, MenuTokens};
    pub use crate::select::{
        Select, SelectOption, SelectOptions, SelectPlugin, SelectStyle, SelectValue, spawn_select,
    };
    pub use crate::separator::{SeparatorAxis, SeparatorPlugin, SeparatorStyle, spawn_separator};
    pub use crate::setting_row::{
        SettingRow, SettingRowControl, SettingRowDescription, SettingRowLabel, SettingRowPlugin,
        SettingRowSpec, SettingRowStyle, SettingRowTokens, SettingSection, spawn_section_header,
        spawn_setting_row,
    };
    pub use crate::text_input::{
        SubmitText, TextInput, TextInputBuffer, TextInputContents, TextInputFilter,
        TextInputHandle, TextInputMode, TextInputNode, TextInputPrompt, TextInputStyle,
        TextInputStylePlugin, spawn_text_input,
    };
    pub use crate::theme::{ColorPalette, FontHandle, Spacing, ThemePlugin, Typography};
    pub use crate::toast::{
        Toast, ToastConfig, ToastDismissible, ToastDuration, ToastExternalProgress, ToastMessage,
        ToastNodes, ToastPlugin, ToastPosition, ToastShowProgress, ToastSlide, ToastSpec,
        ToastTitle, ToastVariant, spawn as spawn_toast, spawn_error as spawn_toast_error,
    };
    pub use crate::tooltip::{Tooltip, TooltipArrow, TooltipPlugin, TooltipPopup, TooltipPosition};
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
        add_once::<card::CardPlugin>(app, || card::CardPlugin);
        add_once::<checkbox::CheckboxStylePlugin>(app, || checkbox::CheckboxStylePlugin);
        add_once::<switch::SwitchStylePlugin>(app, || switch::SwitchStylePlugin);
        add_once::<toggle_group::ToggleGroupStylePlugin>(app, || {
            toggle_group::ToggleGroupStylePlugin
        });
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
