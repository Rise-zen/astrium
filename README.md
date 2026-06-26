# astrium

Wallpaper-driven Material You theming for Linux. Point it at an image and it
extracts a vibrant accent, builds a coherent palette, writes per-app config
files, and pushes a live reload to every running app that can take one.

```bash
astrium apply ~/Wallpaper/forest.jpg
# or just (legacy positional form)
astrium ~/Wallpaper/forest.jpg
```

Want it to retheme automatically whenever you swap the wallpaper? Run the
native watch daemon:

```bash
astrium watch --interval 200 &
```

## What it does

- Extracts a source color from the image (Material You quantization + Score)
- Builds a Material You light/dark scheme, then re-saturates accents so they
  read clearly in status bars and overlays
- Writes:
  - `~/.cache/astrium/colors.json` — wal-compatible palette
  - `~/.cache/astrium/colors-kitty.conf` — kitty terminal theme
  - `~/.cache/astrium/colors-hyprland.conf` — Hyprland window border colors
  - `~/.cache/astrium/nvim-theme.lua` — Neovim highlight overrides
  - `/tmp/qs_colors.json` — Catppuccin-named palette for [quickshell](https://quickshell.outfoxxed.me/) configs
- Sets the wallpaper via [`awww`](https://github.com/LGFae/awww)
- Pushes an instant reload to every running Neovim instance via a socket
  registered under `~/.cache/astrium/nvim-sockets/`
- Patches `~/.config/cava/config` in-place (between `# >>> astrium` markers)

## Install

### Nix (no clone, no PATH wrangling)

```sh
nix run github:zerkal-beta/astrium -- apply ~/Wallpaper/sunset.jpg
nix run github:zerkal-beta/astrium#watch

# or permanently
nix profile install github:zerkal-beta/astrium
```

home-manager / NixOS module:

```nix
{ inputs, ... }: {
  imports = [ inputs.astrium.homeManagerModules.default ];

  services.astrium = {
    enable   = true;
    watch    = true;   # installs a systemd user service
    interval = 200;    # poll milliseconds
  };
}
```

### Arch (manual)

```sh
git clone https://github.com/zerkal-beta/astrium
cd astrium
cargo build --release
install -m755 target/release/astrium ~/.local/bin/
```

## Config

Optional `~/.config/astrium/config.toml`:

```toml
[theme]
mode = "dark"          # "dark" | "light"
bg_darken = 0.4
fg_mute = 0.7
ansi_mute = 0.55

[outputs]
kitty = true
hyprland = true
nvim = true
cava = true
quickshell = true
```

All defaults are `true` / sensible.

## Neovim hookup

Add to `init.lua`:

```lua
local astrium_theme_path = vim.fn.expand("~/.cache/astrium/nvim-theme.lua")

local function apply_astrium_theme()
  if not vim.loop.fs_stat(astrium_theme_path) then return end
  local ok, theme = pcall(dofile, astrium_theme_path)
  if ok and theme and theme.apply then theme.apply() end
end

apply_astrium_theme()
_G.AstriumReload = apply_astrium_theme

local sock_dir = vim.fn.expand("~/.cache/astrium/nvim-sockets")
vim.fn.mkdir(sock_dir, "p")
local sock = sock_dir .. "/" .. vim.fn.getpid() .. ".sock"
vim.fn.serverstart(sock)

vim.api.nvim_create_autocmd("VimLeavePre", {
  callback = function() pcall(vim.fn.delete, sock) end,
})
```

## Library

`src/lib.rs` exposes `astrium::apply()` and `astrium::current_wallpaper()` so
other Rust apps can drive the same pipeline without spawning a process.

## Companions

- [zerkal](https://github.com/zerkal-beta/zerkal) — quickshell overlay with
  orbiting album covers that consumes the same palette
- [lyrics](https://github.com/Rise-zen/lyrics) — terminal lyrics display
  themed from astrium's `colors.json`

## License

MIT
