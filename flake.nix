{
  description = "Modrinth's game launcher";

  inputs.nixpkgs.url = "nixpkgs/nixpkgs-unstable";

  outputs = {
    nixpkgs,
    self,
    ...
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    forSystem = system: fn:
      fn (import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      });

    forAllSystems = fn: nixpkgs.lib.genAttrs systems (system: forSystem system fn);
  in {
    checks = forAllSystems ({
      pkgs,
      lib,
      ...
    }: {
      rustfmt =
        pkgs.runCommand "check-rustfmt" {
          nativeBuildInputs = with pkgs; [cargo rustfmt];
        } ''
          cd ${./.}
          cargo fmt -- --check
          touch $out
        '';

      alejandra = pkgs.runCommand "check-alejandra" {} ''
        ${lib.getExe pkgs.alejandra} --check ${./.}
        touch $out
      '';
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          # base toolchain
          cargo
          rustc

          # toolchain utils
          clippy
          rustfmt
          rust-analyzer

          # nix utils
          self.formatter.${pkgs.system}
          nil
        ];

        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      };
    });

    formatter = forAllSystems (pkgs: pkgs.alejandra);

    overlays.default = final: prev: {
      modrinth-app-unwrapped = prev.callPackage ./nix {
        version = builtins.substring 0 7 self.rev or "dirty";

        inherit
          (final.darwin.apple_sdk.frameworks)
          AppKit
          CoreServices
          Security
          WebKit
          ;

        inherit (final.nodePackages) pnpm;
      };

      modrinth-app = prev.callPackage ./nix/wrapper.nix {
        inherit (final) modrinth-app-unwrapped;
      };
    };

    packages = forAllSystems (pkgs: let
      pkgs' = self.overlays.default (pkgs // pkgs') pkgs;
    in {
      inherit
        (pkgs')
        modrinth-app-unwrapped
        modrinth-app
        ;

      default = pkgs'.modrinth-app;
    });
  };
}
