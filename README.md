# astrium

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
| `~/.cache/astrium/colors-hyprland.conf` | Hyprland border colours — auto-applied via `hyprctl keyword` |
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
nix run github:zerkal-beta/astrium -- apply ~/Wallpaper/sunset.jpg
nix run github:zerkal-beta/astrium#watch

# permanent
nix profile install github:zerkal-beta/astrium
```

home-manager / NixOS module:

```nix
{ inputs, ... }: {
  imports = [ inputs.astrium.homeManagerModules.default ];

  services.astrium = {
    enable   = true;
    watch    = true;     # registers a systemd user service for the watcher
    interval = 200;      # poll milliseconds
  };
}
```

The module wires `astrium watch` into `graphical-session.target`, so it
starts when your compositor is up and restarts automatically on failure.

### Arch / manual

```sh
git clone https://github.com/zerkal-beta/astrium
cd astrium
cargo build --release
install -m755 target/release/astrium ~/.local/bin/
```

Runtime dependencies (most are optional, only needed for the matching
output):

- [`awww`](https://github.com/LGFae/awww) — wallpaper backend
- `kitty` — for live colour-reload of the terminal
- `hyprctl` — for live Hyprland border-reload
- `neovim` — for live theme push (via `nvim --remote-expr`)
- `cava` — for the spectrum gradient

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

## CLI

```text
Usage: astrium [COMMAND]

Commands:
  apply  Apply a wallpaper one-shot: set it via awww + write all palette files
  watch  Poll awww for wallpaper changes and re-theme automatically
```

`astrium <path>` (no subcommand) is kept as a legacy alias for
`astrium apply <path>`.

Watch flags:

- `--interval <ms>` — poll cadence, default `250`.

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
astrium = { git = "https://github.com/zerkal-beta/astrium" }
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

## License

MIT — see [LICENSE](LICENSE).
