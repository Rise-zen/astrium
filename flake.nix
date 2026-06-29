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
          };

          config = lib.mkIf cfg.enable {
            # Bundle the binary into the user's environment so `astrium X.jpg`
            # works in any shell.
            home.packages = lib.optional (config ? home) cfg.package;

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
      in {
        packages.default = astrium;
        packages.astrium = astrium;

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
