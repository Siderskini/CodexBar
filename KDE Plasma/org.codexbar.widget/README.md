# CodexBar Plasma Widget (Plasma 6)

This folder contains a KDE Plasma applet package that consumes JSON from `codexbar-service`.

## Install for testing

```bash
kpackagetool6 --type Plasma/Applet --install "KDE Plasma/org.codexbar.widget"
```

If already installed:

```bash
kpackagetool6 --type Plasma/Applet --upgrade "KDE Plasma/org.codexbar.widget"
```

Then add `CodexBar` from Plasma's widget picker.

## Remove

```bash
kpackagetool6 --type Plasma/Applet --remove org.codexbar.widget
```

## Service dependency

The applet shell command defaults to:

```bash
codexbar-service snapshot --from-codexbar-cli --provider all --status
```

Override it from widget settings if your binary path differs.
