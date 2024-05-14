{
  lib,
  stdenv,
  symlinkJoin,
  modrinth-app-unwrapped,
  wrapGAppsHook,
  addOpenGLRunpath,
  flite,
  glib-networking,
  jdk8,
  jdk17,
  jdks ? [jdk8 jdk17],
  libGL,
  libpulseaudio,
  udev,
  xorg,
}:
symlinkJoin {
  name = "modrinth-app-${modrinth-app-unwrapped.version}";

  paths = [modrinth-app-unwrapped];

  buildInputs = [
    glib-networking
  ];

  nativeBuildInputs = [
    wrapGAppsHook
  ];

  postBuild = let
    libPath = lib.makeLibraryPath [
      flite # narrator support
      libGL
      libpulseaudio
      stdenv.cc.cc.lib

      udev # oshi

      # lwjgl
      xorg.libX11
      xorg.libXcursor
      xorg.libXext
      xorg.libXxf86vm
      xorg.libXrandr
    ];

    args =
      ["--prefix PATH : ${lib.makeSearchPath "bin/java" jdks}"]
      ++ lib.optionals stdenv.isLinux [
        "--set LD_LIBRARY_PATH ${addOpenGLRunpath.driverLink}/lib:${libPath}"
        "--prefix PATH : ${lib.makeBinPath [xorg.xrandr]}"
      ];
  in ''
    gappsWrapperArgs+=(
      ${lib.concatLines args}
    )

    wrapGAppsHook
  '';

  inherit (modrinth-app-unwrapped) meta;
}
