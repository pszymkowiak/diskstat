# diskstat

Fast TUI disk usage analyzer built in Rust. A modern alternative to WinDirStat, ncdu, and Disk Inventory X.

![diskstat](screenshots/diskstat.png)

[Francais](#francais) | [English](#english)

---

<a name="english"></a>
## English

### Why diskstat?

diskstat combines the best features of existing disk usage analyzers with modern performance:

- **vs ncdu**: 10x faster scanning via parallel walk + macOS acceleration, instant cache reload
- **vs WinDirStat**: Native terminal UI, cross-platform, keyboard-first navigation
- **vs Disk Inventory X**: Open source, actively maintained, modern Rust safety
- **vs dust**: Interactive TUI with tree navigation, treemap visualization, duplicate detection
- **vs gdu**: Feature parity (treemap, duplicates, search) with better ergonomics

### Features

- **Parallel scanner** with macOS `getattrlistbulk` acceleration
- **Instant restart** via binary tree cache (< 100ms for cached scans)
- **Interactive treemap** with color-coded file extensions
- **File tree** with size bars, percentages, expand/collapse
- **Duplicate detection** (3-pass: size > partial hash > full hash with blake3)
- **Extension statistics** tab
- **Keyboard + mouse** navigation
- **Search** (vim-style `/`)
- **Split pane** with draggable separator
- **Multiple themes** (cycle with `t`)
- **Export to CSV**
- **Safe**: symlinks skipped, delete restricted to scan root, 10M node OOM guard

### Installation

#### From releases (recommended)

```bash
# macOS (Apple Silicon)
curl -L https://github.com/pszymkowiak/diskstat/releases/latest/download/diskstat-aarch64-apple-darwin.tar.gz | tar xz
sudo mv diskstat /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/pszymkowiak/diskstat/releases/latest/download/diskstat-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv diskstat /usr/local/bin/
```

#### From source

```bash
cargo install --git https://github.com/pszymkowiak/diskstat
```

### Usage

```bash
diskstat                  # Scan current directory (or last scanned)
diskstat /path/to/dir     # Scan specific directory
diskstat ~/Downloads      # Scan Downloads
diskstat -f /path         # Force fresh scan (ignore cache)
diskstat --info           # Show version info
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Up/Down` / `j/k` | Navigate tree |
| `Left/Right` / `h/l` | Collapse/Expand |
| `Enter` | Zoom into directory (treemap) |
| `Backspace` | Zoom out (treemap) |
| `Tab` | Switch pane (tree/treemap) |
| `1` `2` `3` | Switch tab (TreeMap/Extensions/Duplicates) |
| `/` | Search (vim-style) |
| `n` / `N` | Next/Previous search match |
| `r` | Rescan current directory |
| `R` | Rescan subtree |
| `p` | Change path |
| `o` | Open in Finder/file manager |
| `c` | Copy path to clipboard |
| `e` | Export to CSV |
| `d` | Find duplicates |
| `D` | Delete selected (with confirmation) |
| `t` | Cycle theme |
| `m` | Toggle treemap visibility |
| `?` | Help |

### Performance

| Metric | Value |
|--------|-------|
| Scanner | Parallel (rayon) + macOS getattrlistbulk |
| Cache load | < 100ms for 1M+ files |
| Memory | Arena-based tree, interned extensions |
| Treemap | Single-pass render, pre-allocated buffers |
| Duplicates | 3-pass with blake3 (size > 4KB hash > full hash) |

---

<a name="francais"></a>
## Francais

### Fonctionnalites

- **Scanner parallele** avec acceleration macOS `getattrlistbulk`
- **Redemarrage instantane** via cache binaire (< 100ms pour les scans en cache)
- **Treemap interactif** avec couleurs par extension de fichier
- **Arbre de fichiers** avec barres de taille, pourcentages, deplier/replier
- **Detection de doublons** (3 passes : taille > hash partiel > hash complet avec blake3)
- **Onglet statistiques** par extension
- **Navigation clavier + souris**
- **Recherche** (style vim `/`)
- **Panneau divise** avec separateur deplacable
- **Themes multiples** (changer avec `t`)
- **Export CSV**
- **Securise** : symlinks ignores, suppression restreinte au repertoire scanne, garde OOM 10M noeuds

### Installation

#### Depuis les releases (recommande)

```bash
# macOS (Apple Silicon)
curl -L https://github.com/pszymkowiak/diskstat/releases/latest/download/diskstat-aarch64-apple-darwin.tar.gz | tar xz
sudo mv diskstat /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/pszymkowiak/diskstat/releases/latest/download/diskstat-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv diskstat /usr/local/bin/
```

#### Depuis les sources

```bash
cargo install --git https://github.com/pszymkowiak/diskstat
```

### Utilisation

```bash
diskstat                  # Scanner le repertoire courant (ou le dernier scanne)
diskstat /chemin/vers/dir # Scanner un repertoire specifique
diskstat ~/Downloads      # Scanner Downloads
diskstat -f /chemin       # Forcer un scan frais (ignorer le cache)
diskstat --info           # Afficher les infos de version
```

### Raccourcis clavier

| Touche | Action |
|--------|--------|
| `q` / `Ctrl+C` | Quitter |
| `Haut/Bas` / `j/k` | Naviguer dans l'arbre |
| `Gauche/Droite` / `h/l` | Replier/Deplier |
| `Entree` | Zoomer dans le repertoire (treemap) |
| `Retour` | Dezoomer (treemap) |
| `Tab` | Changer de panneau (arbre/treemap) |
| `1` `2` `3` | Changer d'onglet (TreeMap/Extensions/Doublons) |
| `/` | Rechercher (style vim) |
| `n` / `N` | Resultat suivant/precedent |
| `r` | Rescanner le repertoire courant |
| `R` | Rescanner le sous-arbre |
| `p` | Changer de chemin |
| `o` | Ouvrir dans le Finder/gestionnaire de fichiers |
| `c` | Copier le chemin dans le presse-papiers |
| `e` | Exporter en CSV |
| `d` | Chercher les doublons |
| `D` | Supprimer (avec confirmation) |
| `t` | Changer de theme |
| `m` | Afficher/masquer le treemap |
| `?` | Aide |

### Performance

| Metrique | Valeur |
|----------|--------|
| Scanner | Parallele (rayon) + macOS getattrlistbulk |
| Chargement cache | < 100ms pour 1M+ fichiers |
| Memoire | Arbre arena, extensions internees |
| Treemap | Rendu single-pass, buffers pre-alloues |
| Doublons | 3 passes avec blake3 (taille > hash 4Ko > hash complet) |

---

## Architecture

diskstat is built with a clean separation of concerns:

- **Scanner** (`src/scanner/`): Parallel filesystem walker using rayon work-stealing, macOS `getattrlistbulk` for 10x speedup
- **Arena tree** (`src/types.rs`): Indextree-based memory-efficient storage with interned extensions
- **Treemap** (`src/treemap_algo.rs`): Squarified treemap algorithm optimized for terminal aspect ratio
- **TUI** (`src/ui/`): Ratatui widgets with manual buffer rendering for treemap, i18n support (EN/FR)
- **Cache** (`src/scanner/cache.rs`, `tree_cache.rs`): SQLite directory cache + binary tree serialization for <100ms startup
- **Duplicates** (`src/scanner/dupes.rs`): 3-pass parallel deduplication (size > partial hash > full blake3)

Key optimizations:
- Zero-copy sorted children with RefCell cache
- Thread-local buffers for hash computation
- Pre-allocated hit regions for mouse click detection
- Single-pass treemap rendering with background fill + label pass

## Contributing

Contributions welcome! Please:

1. Run tests before submitting: `cargo test`
2. Format code: `cargo fmt`
3. Run clippy: `cargo clippy --all-targets`
4. Add tests for new features
5. Update README for user-facing changes

## License

MIT
