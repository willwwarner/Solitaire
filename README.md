# Solitaire

Goal: An app to play (many) patience games, designed for the GNOME Desktop

### To-Do:
 * Implement more games
 * Add sounds
 * Add animations
 * Add scoring
 * Add preferences
 * Add more card themes

## Try it out
### Download and Install a CI artifact (x86_64 only)

If you have an x86_64 machine, you can download and install a CI artifact:
```bash
curl https://gitlab.gnome.org/api/v4/projects/34657/jobs/artifacts/main/raw/org.gnome.gitlab.wwarner.Solitaire.Devel.flatpak?job=flatpak@x86_64 --output solitaire.flatpak
flatpak install solitaire.flatpak
```
This requires you to manually update by uninstalling, re-downloading, and installing.

### Build from source

Building and running using Foundry:

```bash
git clone https://gitlab.gnome.org/wwarner/solitaire.git
cd solitaire
foundry run
```

