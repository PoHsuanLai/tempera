# The theme is a function

**Status:** proposed
**Scope:** `tempera-widgets`' token resources, and every widget that reads them

## The problem

A UI assigns numbers to thousands of visual quantities. None of them has a
correct value in isolation — 16px of padding is neither good nor bad, only
good or bad *relative to* the text beside it and the container around it.

So the problem is not "what number?". It is:

> Given thousands of interdependent quantities, how do you assign values such
> that the **relationships** between them stay consistent?

Everything below is machinery for maintaining relationships. None of it finds
numbers.

## What is wrong today

Measured across `tempera-widgets`: **74 hardcoded pixel literals against 18
token references.**

The two surfaces that prompted this — the settings dialog and the command
palette — are the worst offenders. `dialog` is 12 hardcoded values and **zero**
token references; it does not read `Spacing` at all. `command` is 13 to 4.

They were each tuned by eye, separately, and they disagree:

| | radius | row height | horizontal padding |
| --- | --- | --- | --- |
| dialog | 12 | 44 (title bar) | 18 |
| command palette | 6 | 36 (item) | 8 |
| context menu | 4 | `menu.item_height` | `menu.item_padding_x` |

Neither radius is wrong. Nobody ever decided.

Eye-tuning solves for local appearance and cannot maintain global
relationships. That is the entire defect, and it is why the two look *almost*
aligned rather than either matching or obviously differing.

The newer per-widget `*Tokens` resources reproduce the same defect in tidier
form: `ListRowTokens` hardcodes `padding_x: 24.0, corner_radius: 2.0` while
*injecting `Spacing` and not using it for them*.

## Three constraints, from three unrelated places

These are often presented as competing schools of design thought. They are not
schools; nobody adopted them as positions. They are three facts about eyes,
screens and hands that happen to constrain the same numbers.

**Perception is ratio-based.** The smallest detectable difference is a constant
*fraction* of the stimulus, not a constant amount (Weber 1834, Fechner 1860).
4px and 8px look obviously different; 100px and 104px look identical. Hence
spacing scales are geometric, for the same reason audio uses decibels.

**Screens quantise.** At fractional display scaling (0.75×, 1.5×) a value must
stay whole or it blurs. 4 is the smallest integer that survives every Android
density bucket.

**Hands and type impose their own minima.** A control must be hittable
(WCAG 2.5.8 AA: 24×24 CSS px) and must fit its own line box. Neither has
anything to do with perceptual spacing.

### How they conflict

They are not merely coexisting — they pull against each other, and the design
is the resolution.

**Geometric ratios vs whole pixels.** A √2 scale gives 4 → 5.657 → 8 → 11.31.
Rounding compounds, because the next step computes from the rounded value.
Resolved by choosing ratios of *small integers*: 3:2 and 4:3, whose product is
exactly 2. From an even base this lands on integers forever, with no rounding
(verified: bases 2, 4, 6, 8, 12 are exact; 3 and 5 go fractional on the first
step).

**Perceptual uniformity vs granularity.** A single 2:1 scale gives 4, 8, 16,
32 — nothing between 8 and 16, which is where most UI spacing lives. Tighten
the ratio and most steps fall below the just-noticeable difference, offering
false choices. Resolved by **two interleaved strands**, each a clean doubling:

```
positions 0,2,4,6:   4,  8, 16, 32     ← base × 2^k
positions 1,3,5,7:   6, 12, 24, 48     ← base × 3/2 × 2^k
```

Twice the granularity, no third ratio. Tim Brown documents double-stranded
scales in [More Meaningful Typography][brown] (*A List Apart*, 2011): they
"include more numbers, and usually fill one another's gaps".

**Magnitude vs position — this one does not resolve, and should not.** Control
heights answer to hit targets and to alignment with neighbouring controls, not
to perceptual spacing. Measured: of the current heights (26, 28, 32, 36, 44),
**none is a member of the scale**, and 26 is not even divisible by any scale
value. No published design system derives control heights; every one declares
them. The correct direction is:

```
height  ← declared from the grid
padding ← solved: (height − line_height) / 2
```

*Not* `height = line_height + 2·padding`, which lets content dictate height and
drifts controls off the grid until they stop aligning.

[brown]: https://alistapart.com/article/more-meaningful-typography/

## The shape: a function from a small config

A theme is not a table of numbers. It is a function:

```rust
ThemeConfig  ->  Tokens
```

Three inputs; everything else derived. "Coherent" then stops being an
aspiration and becomes a property that can be tested over the whole input
space.

### The three inputs, and why exactly three

Each input is an independent **source of constraint**. Independence is what
makes it an input rather than a derivation, and each claim below was tested,
not assumed.

| input | constraint source | why it cannot be derived |
| --- | --- | --- |
| `base` | display scaling | nothing else determines it |
| `density` | hit targets, visual weight | heights are integer multiples of base **4** but *fractional* multiples of base **8** (26/8 = 3.25, 36/8 = 4.5) — so the multiplier is a separate choice |
| `text` | font rendering | font sizes divide evenly by neither 4 nor 8 |

The `density` row is load-bearing. If heights followed from `base`, density
would be a function rather than an input.

```rust
pub struct ThemeConfig {
    pub base: Base,
    pub density: Density,
    pub text: FontSize,
}
```

### `base` is validated, not free

```rust
pub struct Base(u8);

impl Base {
    /// Even bases only. An odd base makes the ×3/2 strand fractional on the
    /// first step (3 → 4.5), and a fractional pixel blurs at non-integer
    /// display scaling.
    pub fn new(px: u8) -> Option<Base> { (px % 2 == 0).then_some(Base(px)) }

    pub const FOUR: Base = Base(4);
    pub const EIGHT: Base = Base(8);
}
```

The invariant the theory gives us becomes a constructor, and the reason lives
in the doc comment instead of in somebody's memory.

### `build` can fail, and that is the honest signature

Font size and height are not fully independent: a control cannot be shorter
than its own line box. At font 14, line-height ≈ 20, so a control needs
≥ ~28px. Font 16 at Compact density would clip its own text.

That is a **constraint between two inputs**, not a fourth input, so it is
validation rather than derivation:

```rust
impl ThemeConfig {
    pub fn build(&self) -> Result<Tokens, Incoherent>;
}
```

Some points in the input space are genuinely incoherent. Saying so beats
silently clipping text.

### Density multipliers are per-base, not shared

Measured: `Comfortable = base × (7, 8, 11)` gives 28/32/44 at base 4 — the
values in use today, and all above the 24px AA floor. The same multipliers at
base 8 give **56/64/88**, which is absurd.

So the multiplier table is chosen per base. This is not a flaw in the model; it
is the model correctly reporting that base and density are independent.

## What the type system should and should not do

The survey of prior art (F#, TypeScript, ReasonML, Scala, Haskell, and the Rust
UI ecosystem) found **no shipping UI toolkit that models scale generation in
types**. Two findings shape the restraint below.

**Compose once shipped `DpSquared`, `DpCubed` and `DpInverse`, and deleted
them.** That is the strongest available evidence that a full algebraic lattice
does not pay for itself in UI.

**scalacss independently arrived at omission-as-design** — scalar `*` and `/`,
no `+` between two lengths — but stores the unit as a *runtime field*, so the
omission over-restricts: two px values cannot be added either. A direct
argument for newtype-per-unit, which is what `tutti_types::units` already does.

The only compile-time guarantee anyone actually ships is **category-level**
(a colour cannot go where a length goes). Nobody stops `rem` going where `px`
goes.

### Build

- **`Base`, `Gap`, `Radius`, `ControlHeight`, `FontSize`** as newtypes that do
  not interconvert. No `From<Gap> for ControlHeight` — heights answer to a
  different constraint than gaps, which is exactly the fact that made
  `TITLE_BAR_HEIGHT = 44.0` get hand-copied into a downstream crate.
- **The scale as a method, not fields.** `Spacing::at(Step) -> Gap`, computed.
  A public `xs: f32` field invites assignment, which silently breaks
  proportionality; a method cannot be assigned to.
- **`Radius::concentric_inner(pad)`** — the one operation with a proof.

### Do not build

- **`Step: Add`.** Tested and false: `step(1)` then `step(1)` gives 9, while
  `step(2)` gives 8, because the ratio depends on absolute parity rather than
  relative distance. `Step` is an *index*; it gets `Ord` and named methods
  (`octave_up()`, provably ×2), never arithmetic.
- **`Gap: Add`.** The scale is not closed under addition — 22 of 36 pairwise
  sums fall off it (4+6 = 10). Two stacked gaps give a length, not a `Gap`.
  Same reasoning as `Beat + Beat` in tutti.
- **Any exponent lattice.** See Compose.
- **`Scale<const RATIO>`.** A const can only carry a property of the *code*,
  and the base is a property of the *data* — a host overrides the resource at
  runtime. Same mistake as `AudioIn<const CH>`.

## The one lawful operation

```rust
inner_radius = max(0, outer_radius − padding)
```

A rounded rectangle is the Minkowski sum of a sharp rectangle with a disc of
radius r. Offsetting inward by d shrinks that disc to r − d and **leaves the
arc centres fixed**, so inner and outer stay exactly d apart the whole way
around. When d > r the true offset really is a sharp corner, so the clamp is
correct rather than a fudge. Apple ships this as `ConcentricRectangle`
(WWDC25). Exact for circular arcs; a very good approximation for squircles,
which are not closed under offsetting.

Verified: it **composes** (nesting by 4 then 4 equals nesting by 8 once) and
clamps correctly.

This is the direct fix for the reported symptom. Today `dialog` declares
`CARD_RADIUS = 12.0` and its close button separately declares `4.0` — two
independent guesses. With this, the child asks its parent.

## Honest labelling

Two of the four token groups are theory; two are convention. The code should
say which, because pretending otherwise sends the next reader hunting for a
rule that was never there.

| group | status |
| --- | --- |
| `Spacing` | **generated** — two-strand scale from `base` |
| `Radii` | **generated**, plus `concentric_inner` |
| `Sizing` | **declared** — `base × density`; off-scale deliberately (hit targets, alignment) |
| `Typography` | **curated** — ratios 1.09–1.25, no rule, and correctly so |

Every major system is algorithmic in spacing and curated in type. That is not
inconsistency; type must survive hinting, x-height and optical size, which no
ratio models. Note also that the 4px grid is near-universally applied to
**line-height, not font-size** — all 15 Material 3 line-heights divide by 4;
the sizes (57/45/22/14/11) do not.

## Property tests

The laws, not examples:

```rust
#[test] fn an_octave_is_exactly_a_doubling()          // at(n+2) == 2·at(n)
#[test] fn every_valid_base_yields_whole_pixels()     // over the input space
#[test] fn nesting_twice_equals_nesting_once()        // concentric composes
#[test] fn a_radius_never_goes_negative()             // clamp
#[test] fn an_odd_base_is_rejected()                  // constructor invariant
#[test] fn a_control_is_never_shorter_than_its_text() // build() rejects
```

## Migration

Staged, because one step is visible and the rest are not.

1. **Add the types and `build`.** Additive; nothing changes on screen.
2. **Point the ~35 spacing literals at existing scale values.** Pure
   substitution — 4, 8, 12, 16, 24 are already scale members. Invisible.
3. **Retire the ~15 per-widget size constants onto `Sizing`.** *Visible*:
   36 → 32, 26 → 28. Snapping to the scale is the point, and things move by a
   few pixels.
4. **Rebase the three `*Tokens` defaults on the scales** rather than restating
   numbers.

Step 3 is the only one that changes appearance and should be reviewed on its
own.

## Consequences to accept

**The theme becomes rebuildable at runtime.** That is a feature — a density
setting is a normal thing to want — but it means `Tokens` must be *re-read* by
widgets rather than baked in at spawn. Several widgets currently read tokens
once during `spawn_*`; changing base would need a repaint pass or a respawn.
This follows from making the config user-settable, not from the theory.

**This does not claim the numbers are right.** `base = 4` is convention.
`control_md = 32` is convention. The model says where each number *comes from*
— which is a weaker claim than "these are the correct values", and the only one
the evidence supports.

## What this deliberately does not solve

A complete theory would map content and context to a number: that this is a
dialog title bar, beside a close button, above a scrollable list, for
professionals in long sessions. Nothing in perception research operates at that
level.

Much of what "looks right" is also learned convention rather than perception —
users expect a dialog to look like the dialogs they have used — and that shifts
every few years. No perceptual theory predicts a trend.

So: a thin layer of real perceptual results, a thicker layer of engineering
constraints, and a large layer of convention doing most of the work. The value
here is not rigour. It is that `dialog` currently reads zero tokens and
hand-tunes twelve values, and that is why it and the palette do not match.
