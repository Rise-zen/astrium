{
  description = "astrium — wallpaper-driven Material You theming for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    let
      # NixOS / home-manager module — same options regardless of which one
      # imports it. Exposes `services.astrium.*` for declarative install.
      mkModule = pkgs: { config, lib, ... }:
        let
          cfg = config.services.astrium;
          astriumPkg = self.packages.${pkgs.system}.default;
          isHome = config ? home;
          tomlFormat = pkgs.formats.toml { };

          # Map the typed Nix options onto astrium's config.toml schema. Only
          # the snake_case keys the Rust side reads are emitted; templates carry
          # their input through the nix store so the source is immutable and
          # reproducible while astrium renders the output at runtime.
          tomlConfig = {
            theme = {
              mode = cfg.theme.mode;
              bg_darken = cfg.theme.bgDarken;
              fg_mute = cfg.theme.fgMute;
              ansi_mute = cfg.theme.ansiMute;
            };
            outputs = {
              inherit (cfg.outputs) kitty hyprland nvim cava quickshell;
            };
            templates = map (t: {
              input = toString t.input;
              inherit (t) output;
            }) cfg.templates;
          };
        in {
          options.services.astrium = {
            enable = lib.mkEnableOption "astrium wallpaper-driven theming daemon";

            package = lib.mkOption {
              type = lib.types.package;
              default = astriumPkg;
              description = "astrium package to use.";
            };

            watch = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = ''
                Run `astrium watch` as a systemd user service that polls
                awww and re-themes on every wallpaper change.
              '';
            };

            interval = lib.mkOption {
              type = lib.types.int;
              default = 250;
              description = "Poll interval in milliseconds for the watch daemon.";
            };

            theme = {
              mode = lib.mkOption {
                type = lib.types.enum [ "dark" "light" ];
                default = "dark";
                description = "Which Material You scheme to derive.";
              };
              bgDarken = lib.mkOption {
                type = lib.types.float;
                default = 0.4;
                description = "0..1 — extra darkening applied to the background.";
              };
              fgMute = lib.mkOption {
                type = lib.types.float;
                default = 0.7;
                description = "0..1 — pull the foreground toward grey.";
              };
              ansiMute = lib.mkOption {
                type = lib.types.float;
                default = 0.55;
                description = "0..1 — mute the 16 ansi roles.";
              };
            };

            outputs = {
              kitty = lib.mkOption { type = lib.types.bool; default = true; description = "Reload kitty colours."; };
              hyprland = lib.mkOption { type = lib.types.bool; default = true; description = "Reload Hyprland border/shadow colours."; };
              nvim = lib.mkOption { type = lib.types.bool; default = true; description = "Write Neovim theme + notify running instances."; };
              cava = lib.mkOption { type = lib.types.bool; default = true; description = "Patch cava gradient."; };
              quickshell = lib.mkOption { type = lib.types.bool; default = true; description = "Write /tmp/qs_colors.json for quickshell bars."; };
            };

            templates = lib.mkOption {
              default = [ ];
              description = ''
                User templates rendered on every retheme. `input` is a file
                containing `{{placeholder}}`s (carried through the nix store);
                `output` is where the rendered result is written.
              '';
              example = lib.literalExpression ''
                [ { input = ./waybar/colors.css.in; output = "~/.config/waybar/colors.css"; } ]
              '';
              type = lib.types.listOf (lib.types.submodule {
                options = {
                  input = lib.mkOption {
                    type = lib.types.path;
                    description = "Template source file with {{var}} placeholders.";
                  };
                  output = lib.mkOption {
                    type = lib.types.str;
                    description = "Destination path for the rendered file (supports a leading ~).";
                  };
                };
              });
            };
          };

          config = lib.mkIf cfg.enable {
            # Bundle the binary into the user's environment so `astrium X.jpg`
            # works in any shell.
            home.packages = lib.optional isHome cfg.package;

            # Generate ~/.config/astrium/config.toml from the typed options so
            # the whole theme is declared in Nix. Home-manager only — NixOS has
            # no per-user xdg.configFile surface.
            xdg.configFile = lib.mkIf isHome {
              "astrium/config.toml".source =
                tomlFormat.generate "astrium-config.toml" tomlConfig;
            };

            # systemd user service — only registered if the user actually
            # wants the watcher running, otherwise astrium stays one-shot CLI.
            systemd.user.services.astrium = lib.mkIf cfg.watch {
              Unit = {
                Description = "astrium — auto-retheme on wallpaper change";
                PartOf = [ "graphical-session.target" ];
                After = [ "graphical-session.target" ];
              };
              Service = {
                ExecStart = "${cfg.package}/bin/astrium watch --interval ${toString cfg.interval}";
                Restart = "on-failure";
                RestartSec = 2;
              };
              Install.WantedBy = [ "graphical-session.target" ];
            };
          };
        };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        astrium = pkgs.rustPlatform.buildRustPackage {
          pname   = "astrium";
          version = "0.1.0";
          src     = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];

          # awww (user's swww fork) is invoked at runtime; we don't pin it
          # because users will have their own build path. Just make sure bash
          # is in PATH for the awww subprocess.
          postInstall = ''
            wrapProgram $out/bin/astrium \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.bash ]}
          '';

          meta = with pkgs.lib; {
            description = "Wallpaper-driven Material You theming for kitty, Hyprland, Neovim";
            homepage    = "https://github.com/Rise-zen/astrium";
            license     = licenses.mit;
            platforms   = platforms.linux;
            mainProgram = "astrium";
          };
        };

        # Build-time palette: run `astrium generate` inside a sandbox to bake
        # the color files for a fixed wallpaper into a derivation. No daemon,
        # no awww — the theme is computed once at build and lives in the store.
        #   astrium.lib.${system}.mkPalette ./wallpaper.jpg
        # produces a store path containing colors.json, colors-kitty.conf,
        # colors-hyprland.conf, nvim-theme.lua, qs_colors.json, cava-gradient.conf.
        mkPalette = wallpaper:
          pkgs.runCommand "astrium-palette" { } ''
            ${astrium}/bin/astrium generate ${wallpaper} --out $out
          '';
      in {
        packages.default = astrium;
        packages.astrium = astrium;

        lib.mkPalette = mkPalette;

        # `nix run github:Rise-zen/astrium -- apply ~/Wallpaper/sunset.jpg`
        # `nix run github:Rise-zen/astrium#watch`
        apps.default = {
          type    = "app";
          program = "${astrium}/bin/astrium";
          meta    = astrium.meta;
        };
        apps.watch = {
          type    = "app";
          program = "${pkgs.writeShellScript "astrium-watch" ''
            exec ${astrium}/bin/astrium watch "$@"
          ''}";
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rust-analyzer
            pkgs.pkg-config
          ];
        };

        # Re-exported so a flake can do
        #   imports = [ astrium.homeManagerModules.default ];
        homeManagerModules.default = mkModule pkgs;
        nixosModules.default       = mkModule pkgs;
      });
}
