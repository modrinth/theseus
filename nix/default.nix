{
  lib,
  version,
  stdenv,
  stdenvNoCC,
  fetchFromGitHub,
  rustPlatform,
  buildGoModule,
  makeDesktopItem,
  copyDesktopItems,
  AppKit,
  CoreServices,
  Security,
  WebKit,
  cacert,
  pnpm,
  esbuild,
  dbus,
  freetype,
  gtk3,
  jq,
  libappindicator-gtk3,
  librsvg,
  libsoup,
  moreutils,
  openssl,
  pkg-config,
  webkitgtk,
}:
rustPlatform.buildRustPackage rec {
  pname = "modrinth-app-unwrapped";
  inherit version;

  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  pnpm-deps = stdenvNoCC.mkDerivation {
    pname = "${pname}-pnpm-deps";
    inherit src version;

    nativeBuildInputs = [
      cacert
      jq
      moreutils
      pnpm
    ];

    # https://github.com/NixOS/nixpkgs/blob/763e59ffedb5c25774387bf99bc725df5df82d10/pkgs/applications/misc/pot/default.nix#L56
    installPhase = ''
      export HOME=$(mktemp -d)

      cd theseus_gui
      pnpm config set store-dir $out
      pnpm install --frozen-lockfile --no-optional --ignore-script

      rm -rf $out/v3/tmp
      for f in $(find $out -name "*.json"); do
        sed -i -E -e 's/"checkedAt":[0-9]+,//g' $f
        jq --sort-keys . $f | sponge $f
      done
    '';

    dontFixup = true;
    outputHashMode = "recursive";
    outputHash = "sha256-gRQfWrAY/2XxiVSHtQd4YKruJWjkpAB5OsXZMmV0iDs=";
  };

  buildInputs =
    [openssl]
    ++ lib.optionals stdenv.isLinux [
      dbus
      freetype
      gtk3
      libappindicator-gtk3
      librsvg
      libsoup
      webkitgtk
    ]
    ++ lib.optionals stdenv.isDarwin [
      AppKit
      CoreServices
      Security
      WebKit
    ];

  nativeBuildInputs = [
    pkg-config
    pnpm
    copyDesktopItems
  ];

  ESBUILD_BINARY_PATH = lib.getExe (
    esbuild.override {
      buildGoModule = args:
        buildGoModule (args
          // rec {
            version = "0.17.19";
            src = fetchFromGitHub {
              owner = "evanw";
              repo = "esbuild";
              rev = "v${version}";
              hash = "sha256-PLC7OJLSOiDq4OjvrdfCawZPfbfuZix4Waopzrj8qsU=";
            };
            vendorHash = "sha256-+BfxCyg0KkDQpHt/wycy/8CTG6YBA/VJvJFhhzUnSiQ=";
          });
    }
  );

  preBuild = ''
    export HOME=$(mktemp -d)
    export STORE_PATH=$(mktemp -d)
    pushd theseus_gui

    cp -r ${pnpm-deps}/* "$STORE_PATH"
    chmod -R +w "$STORE_PATH"

    pnpm config set store-dir "$STORE_PATH"
    pnpm install --offline --frozen-lockfile --no-optional --ignore-script
    pnpm build

    popd
  '';

  desktopItems = [
    (makeDesktopItem rec {
      name = "com.modrinth.ModrinthApp";
      exec = "theseus_gui";
      icon = "com.modrinth.ModrinthApp";
      desktopName = "Modrinth App";
      genericName = desktopName;
      comment = meta.description;
      terminal = false;
      startupNotify = true;
      startupWMClass = "ModrinthApp";
      categories = ["Game" "ActionGame" "AdventureGame" "Simulation"];
      keywords = ["game" "minecraft" "mc"];
    })
  ];

  postInstall = lib.optionalString stdenv.isLinux ''
    mkdir -p $out/share/{applications,icons/hicolor/256x256/apps}
    copyDesktopItems
    cp theseus_gui/src-tauri/icons/Square284x284Logo.png $out/share/icons/hicolor/256x256/apps/com.modrinth.ModrinthApp.png
  '';

  meta = with lib; {
    mainProgram = "theseus_gui";
    description = "Modrinth's game launcher";
    longDescription = ''
      Modrinth's game launcher which can be used as a CLI, GUI, and a library for creating and playing Modrinth projects.
    '';
    homepage = "https://modrinth.com";
    license = with licenses; [gpl3Plus unfreeRedistributable];
    maintainers = with maintainers; [getchoo];
    platforms = with platforms; linux ++ darwin;
    # this builds on aarch64, but the launcher itself does not support it yet
    broken = stdenv.isAarch64;
  };
}
