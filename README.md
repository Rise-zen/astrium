# astrium

[![CI](https://github.com/Rise-zen/astrium/actions/workflows/ci.yml/badge.svg)](https://github.com/Rise-zen/astrium/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Wallpaper-driven Material You theming for Linux — extracts a vibrant accent
> from any image, builds a coherent palette, and pushes a live reload to
> every app that knows how to take one (kitty, Hyprland, Neovim, cava,
> quickshell-based bars/overlays). Written in Rust, distributed as a Nix
> flake with a systemd user-service module for declarative installs.

```sh
astrium apply ~/Wallpaper/sunset.jpg     # one-shot
astrium watch --interval 200             # daemon: auto-retheme on change
```

The watch daemon notices when [`awww`](https://github.com/LGFae/awww)
displays a new wallpaper (whether you triggered it via CLI, a wallpaper
picker GUI, or a script) and runs the whole pipeline within ~200 ms.

---

## Pipeline

1. **Extract** a source colour with Material You's own pipeline (Celebi
   quantization → Score ranking). Beats naive pixel-averaging — the result
   is a vibrant accent, not muddy grey.
2. **Build** a Material You light/dark scheme, then re-saturate the accents
   so they read clearly against dark status bars (status-bar greys lift to
   real hues, lightness pinned into a readable mid-range).
3. **Broadcast** the palette to every enabled output in one pass.

### Output files

| Path | Consumer |
|---|---|
| `~/.cache/astrium/colors.json` | wal-style palette (background, foreground, color0..15) |
| `~/.cache/astrium/colors-kitty.conf` | kitty terminal — auto-applied via `kitty @ set-colors --all` |
| `~/.cache/astrium/colors-hyprland.conf` | Hyprland border/shadow colours — auto-applied via `hyprctl keyword` |
| `~/.cache/astrium/nvim-theme.lua` | Neovim highlight overrides |
| `/tmp/qs_colors.json` | Catppuccin-named palette ([quickshell](https://quickshell.outfoxxed.me/) configs poll this) |

Plus side effects:

- Sets the wallpaper via `awww img <path>`.
- Pushes an instant reload to every running Neovim instance through a
  socket registered under `~/.cache/astrium/nvim-sockets/<pid>.sock`.
- Patches `~/.config/cava/config` in-place between `# >>> astrium` /
  `# <<< astrium` markers with a fresh gradient derived from the palette.

---

## Install

### Nix (zero-friction)

```sh
# try it without committing
nix run github:Rise-zen/astrium -- apply ~/Wallpaper/sunset.jpg
nix run github:Rise-zen/astrium#watch

# permanent
nix profile install github:Rise-zen/astrium
```

home-manager / NixOS module:

```nix
{ inputs, ... }: {
  imports = [ inputs.astrium.homeManagerModules.default ];

  services.astrium = {
    enable   = true;
    watch    = true;     # registers a systemd user service for the watcher
    interval = 200;      # poll milliseconds

    # The whole theme is declared here — the module generates
    # ~/.config/astrium/config.toml, so you never hand-write it.
    theme = {
      mode     = "dark";
      bgDarken = 0.4;
      fgMute   = 0.7;
      ansiMute = 0.55;
    };
    outputs.cava = false;   # disable any built-in output

    # Declarative templates: the input lives in the nix store (immutable,
    # reproducible), astrium renders the output at runtime on every retheme.
    templates = [
      { input = ./waybar/colors.css.in; output = "~/.config/waybar/colors.css"; }
      { input = ./rofi/theme.rasi.in;   output = "~/.config/rofi/theme.rasi"; }
    ];
  };
}
```

The module wires `astrium watch` into `graphical-session.target`, so it
starts when your compositor is up and restarts automatically on failure.
`theme`, `outputs` and `templates` are compiled straight into
`config.toml` via `pkgs.formats.toml`, so the entire palette pipeline is
reproducible and lives next to the rest of your Home Manager config.

### Arch / manual (full guide)

**1. Toolchain.** astrium is built with stable Rust (edition 2021):

```sh
sudo pacman -S --needed rust git imagemagick
# or, if you manage Rust via rustup:
rustup default stable
```

`imagemagick` provides the `magick` binary used for cover/accent quantization
and is the one hard runtime dependency.

**2. Build and install the binary:**

```sh
git clone https://github.com/Rise-zen/astrium
cd astrium
cargo build --release
install -Dm755 target/release/astrium ~/.local/bin/astrium
```

**3. Put `~/.local/bin` on your `PATH`** if it isn't already:

```sh
# fish
fish_add_path ~/.local/bin
# bash/zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.profile
```

Verify: `astrium --help` should print the CLI.

**4. Optional runtime deps** — each is only needed for its matching output;
astrium silently skips any that are missing:

| Dependency | Enables |
|---|---|
| [`awww`](https://github.com/LGFae/awww) | wallpaper backend (`astrium apply` sets it, `watch` polls it) |
| `kitty` | live terminal colour-reload via `kitty @ set-colors` |
| `hyprctl` (Hyprland) | live border/shadow reload |
| `neovim` | live theme push via `nvim --remote-expr` |
| `cava` | spectrum gradient (config patched in place) |

**5. First run** — apply any wallpaper to generate every palette file:

```sh
astrium apply ~/Wallpaper/sunset.jpg
```

**6. Run the watcher** so retheming happens automatically on every wallpaper
change. Add to your compositor autostart, e.g. Hyprland:

```lua
-- in your hyprland.lua autostart hook
hl.exec_cmd("astrium watch --interval 200")
```

or the legacy `.conf`:

```conf
exec-once = astrium watch --interval 200
```

---

## Configure

Optional `~/.config/astrium/config.toml`:

```toml
[theme]
mode      = "dark"   # "dark" | "light"
bg_darken = 0.4      # 0..1 — extra darken on the background
fg_mute   = 0.7      # 0..1 — pull foreground toward grey
ansi_mute = 0.55     # 0..1 — mute the 16 ansi roles

[outputs]
kitty      = true
hyprland   = true
nvim       = true
cava       = true
quickshell = true
```

All defaults are sensible; the file is optional.

### Templates — theme any app

The built-in outputs cover the author's setup. To theme anything else, point
astrium at a template file with `{{placeholder}}`s and an output path. Every
template is re-rendered on each wallpaper change:

```toml
[[templates]]
input  = "~/.config/waybar/colors.css.in"
output = "~/.config/waybar/colors.css"

[[templates]]
input  = "~/.config/rofi/theme.rasi.in"
output = "~/.config/rofi/theme.rasi"
```

Inside a template, reference any palette variable:

| Placeholder | Value |
|---|---|
| `{{background}}` / `{{foreground}}` | base colours (`#rrggbb`) |
| `{{color0}}` … `{{color15}}` | the 16 ansi roles |
| `{{cursor}}` | alias of `{{foreground}}` |
| `{{NAME.strip}}` | same colour without the leading `#` (e.g. `{{color4.strip}}` → `89b4fa`, handy for `rgba()`) |

Example `colors.css.in`:

```css
@define-color bg {{background}};
@define-color accent {{color4}};
.border { border: 1px solid #{{color5.strip}}; }
```

Unknown placeholders are left untouched, so a typo is visible in the output
rather than silently blanked.

---

## Neovim hookup

Drop this into `init.lua`:

```lua
local astrium_theme_path = vim.fn.expand("~/.cache/astrium/nvim-theme.lua")

local function apply_astrium_theme()
  if not vim.loop.fs_stat(astrium_theme_path) then return end
  local ok, theme = pcall(dofile, astrium_theme_path)
  if ok and theme and theme.apply then theme.apply() end
end

apply_astrium_theme()

-- Remote-callable entry point — astrium pushes updates here via
-- `nvim --server <sock> --remote-expr 'v:lua.AstriumReload()'`
_G.AstriumReload = apply_astrium_theme

local sock_dir = vim.fn.expand("~/.cache/astrium/nvim-sockets")
vim.fn.mkdir(sock_dir, "p")
local sock = sock_dir .. "/" .. vim.fn.getpid() .. ".sock"
vim.fn.serverstart(sock)

vim.api.nvim_create_autocmd("VimLeavePre", {
  callback = function() pcall(vim.fn.delete, sock) end,
})
```

Every Neovim instance you open will register its own socket; astrium fans
the reload out to all of them on wallpaper change.

---

## Hyprland hookup

Add this to the end of your `hyprland.conf`:

```conf
source = ~/.cache/astrium/colors-hyprland.conf
```

This loads the palette on startup. While running, astrium also applies each
change instantly with `hyprctl keyword`, so borders retint the moment the
wallpaper changes — no reload needed.

---

## CLI

```text
Usage: astrium [COMMAND]

Commands:
  apply     Apply a wallpaper one-shot: set it via awww + write all palette files
  watch     Poll awww for wallpaper changes and re-theme automatically
  generate  Extract a palette and write artifacts to a dir with no side effects
```

`astrium <path>` (no subcommand) is kept as a legacy alias for
`astrium apply <path>`.

Flags:

- `apply --no-wallpaper` — only regenerate the palette; don't touch the wallpaper.
- `watch --interval <ms>` — poll cadence, default `250`.
- `generate <image> --out <dir>` — pure, sandbox-safe; no awww/kitty/hyprctl,
  no `/tmp`, no config patching. Writes every artifact into `<dir>`.

---

## Build-time palettes (Nix)

`generate` is what lets Nix bake a palette at build time — no running daemon,
no wallpaper switch, the theme is computed once and lives in the store:

```nix
let
  palette = astrium.lib.${system}.mkPalette ./wallpaper.jpg;
in {
  # palette is a store path containing colors.json, colors-kitty.conf,
  # colors-hyprland.conf, nvim-theme.lua, qs_colors.json, cava-gradient.conf
  programs.kitty.extraConfig = builtins.readFile "${palette}/colors-kitty.conf";
}
```

Because it runs in the Nix sandbox (no network, no external programs), the
result is fully reproducible: the same wallpaper always yields the same files.

---

## Library

`astrium` is also a small Rust library — `src/lib.rs` exposes:

```rust
pub fn apply(image_path: &Path, cfg: &Config, cache_dir: &Path) -> Result<()>;
pub fn current_wallpaper() -> Option<PathBuf>;
```

So a downstream crate can drive the same pipeline without spawning a
subprocess.

```toml
# Cargo.toml
[dependencies]
astrium = { git = "https://github.com/Rise-zen/astrium" }
```

---

## Family

- **[zerkal](https://github.com/Rise-zen/zerkal-alpha)** — quickshell overlay
  with orbiting album covers; reads `/tmp/qs_colors.json` so the planet,
  the bloom ring, and every accent are driven by the same palette astrium
  emits.
- **[lyrics](https://github.com/Rise-zen/lyrics)** — terminal lyrics display
  that loads `~/.cache/astrium/colors.json` so the current-line colour
  follows your wallpaper too.

---

## Changelog

- **Templates** — `[[templates]]` in the config renders any file with
  `{{placeholder}}`s on every retheme, so astrium can colour apps beyond the
  built-in outputs.
- `astrium apply --no-wallpaper` regenerates the palette without touching the
  wallpaper, for callers that already set the image (sub-second retheme).
- CI: GitHub Actions runs `cargo fmt`/`clippy`/`test` and `nix flake check`.
- Full install guide with step-by-step instructions and dependency table.
- Added Hyprland hookup section.
- Repository URLs moved to `github.com/Rise-zen/astrium`.

---

## License

MIT — see [LICENSE](LICENSE).
