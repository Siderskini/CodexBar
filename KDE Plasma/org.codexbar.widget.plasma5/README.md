# CodexBar Plasma Widget (Plasma 5)

This folder contains a KDE Plasma 5 applet package that consumes JSON from `codexbar-service`.

## Install for testing

```bash
kpackagetool5 --type Plasma/Applet --install "KDE Plasma/org.codexbar.widget.plasma5"
```

If already installed:

```bash
kpackagetool5 --type Plasma/Applet --upgrade "KDE Plasma/org.codexbar.widget.plasma5"
```

Then add `CodexBar` from Plasma's widget picker.

## Remove

```bash
kpackagetool5 --type Plasma/Applet --remove org.codexbar.widget.plasma5
```

## Service dependency

The applet shell command defaults to:

```bash
codexbar-service snapshot --from-codexbar-cli --provider all --status
```

Override it from widget settings if your binary path differs.
