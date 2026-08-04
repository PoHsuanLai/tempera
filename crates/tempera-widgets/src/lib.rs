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
    pub use crate::anim::{EaseTween, Lerpable, SMOOTH_DURATION_SECS, Spring};
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
        ProgressToast, Toast, ToastConfig, ToastDismissible, ToastDuration, ToastMessage,
        ToastNodes, ToastPlugin, ToastPosition, ToastShowProgress, ToastSpec, ToastState,
        ToastTitle, ToastVariant, complete as complete_toast, progress as progress_toast,
        spawn as spawn_toast, spawn_error as spawn_toast_error,
    };
    pub use crate::tooltip::{
        Tooltip, TooltipArrow, TooltipPlugin, TooltipPopup, TooltipPosition, TooltipShortcutFor,
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

#[cfg(test)]
mod styled_widgets_tests {
    use bevy::prelude::*;

    use crate::theme::{Base, StyledNode, ThemeConfig, ThemePlugin};

    /// Every widget that declares a `StyledNode` follows a base change.
    ///
    /// Converted so far: `card`, `checkbox`, `select`, `text_input`,
    /// `toggle_group`. Deliberately not, each for its own reason —
    /// `list_row`/`tree_row`/`setting_row` read host-overridable `*Tokens`
    /// resources that a `StyledNode` would silently bypass (their defaults
    /// should derive from the scale instead); `progress` computes its radius
    /// from its own height, which a step cannot express; `separator` takes a
    /// caller-supplied length; and `slider`/`switch`/`tabs`/`tooltip` animate
    /// `Node` fields every frame and need checking against these writes.
    ///
    /// Spawning each widget properly needs its own `*Style` bundle and a
    /// pile of arguments, so this asserts the property one level down: a
    /// `StyledNode` on any entity resolves against the live theme. What each
    /// widget declares is checked by its own tests; what this pins is that
    /// declaring anything at all is enough to become reactive.
    #[test]
    fn declaring_a_styled_node_is_enough_to_follow_the_theme() {
        let mut app = App::new();
        app.add_plugins(ThemePlugin);

        let e = app
            .world_mut()
            .spawn((
                StyledNode::new().padding_x(crate::theme::Step::new(2)),
                Node::default(),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Node>(e).unwrap().padding.left,
            Val::Px(8.0)
        );

        let coarse = ThemeConfig {
            base: Base::EIGHT,
            ..default()
        };
        app.insert_resource(coarse)
            .insert_resource(coarse.build().expect("base 8 is coherent"));
        app.update();

        assert_eq!(
            app.world().get::<Node>(e).unwrap().padding.left,
            Val::Px(16.0)
        );
    }
}

#[cfg(test)]
mod token_scale_tests {
    use crate::list_row::ListRowTokens;
    use crate::setting_row::SettingRowTokens;
    use crate::theme::{Base, Scale};
    use crate::tree_row::TreeRowTokens;

    #[test]
    fn the_generated_defaults_are_the_shipped_values() {
        // This change is meant to be invisible: the defaults are now named
        // steps rather than literals, and at base 4 they must land on
        // exactly what they always were.
        let l = ListRowTokens::default();
        assert_eq!(
            (
                l.padding_x,
                l.padding_y,
                l.row_gap,
                l.column_gap,
                l.corner_radius
            ),
            (24.0, 8.0, 12.0, 16.0, 2.0)
        );

        let t = TreeRowTokens::default();
        assert_eq!((t.indent_step, t.corner_radius), (8.0, 2.0));
        assert_eq!(
            (t.height, t.icon_size),
            (22.0, 14.0),
            "the strays stayed put"
        );

        let s = SettingRowTokens::default();
        assert_eq!((s.padding_x, s.padding_y, s.row_gap), (24.0, 8.0, 12.0));
        assert_eq!(
            (s.section_gap, s.row_height, s.control_width),
            (18.0, 36.0, 200.0),
            "the off-scale values stayed put"
        );
    }

    #[test]
    fn a_coarser_base_scales_the_generated_fields_and_leaves_the_rest() {
        // What generating the defaults buys: one input moves and the values
        // that answer to the grid follow, while content measures and
        // deliberately off-scale figures do not.
        let coarse = Scale::new(Base::EIGHT);
        let l = ListRowTokens::from_scale(coarse);
        let base4 = ListRowTokens::default();

        assert_eq!(l.padding_x, base4.padding_x * 2.0);
        assert_eq!(l.row_gap, base4.row_gap * 2.0);
        assert_eq!(
            l.trail_min_width, base4.trail_min_width,
            "a content measure does not follow the grid"
        );

        let s = SettingRowTokens::from_scale(coarse);
        assert_eq!(s.padding_y, SettingRowTokens::default().padding_y * 2.0);
        assert_eq!(s.control_width, 200.0);
        assert_eq!(s.row_height, 36.0, "an off-scale height is not swept along");
    }

    #[test]
    fn the_scale_reads_added_in_the_sweep_land_on_their_old_values() {
        // The last pass routed a batch of literals through the scale —
        // command's 4/12/16/24/6/8, number_field's 28 and 8, dialog's close
        // radius 4. Every one is a step, and at the default base each must
        // return exactly what it replaced or the change stopped being
        // invisible.
        use crate::theme::{Scale, Step};
        let s = Scale::new(Base::default());
        let at = |n: i8| s.at(Step::new(n)).get();
        assert_eq!(
            (at(0), at(1), at(2), at(3), at(4), at(5)),
            (4.0, 6.0, 8.0, 12.0, 16.0, 24.0)
        );
    }

    #[test]
    fn overriding_a_token_still_works() {
        // The reason these stay `Resource`s with public fields. Generating a
        // default must not turn a tunable into a fixed value — a host that
        // wants a denser row still writes one.
        let l = ListRowTokens {
            padding_x: 6.0,
            ..Default::default()
        };
        assert_eq!(l.padding_x, 6.0);
        assert_eq!(l.row_gap, 12.0, "the rest still comes from the scale");
    }
}
