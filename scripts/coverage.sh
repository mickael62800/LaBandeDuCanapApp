#!/usr/bin/env bash
# Couverture de tests du depot : Rust (cargo-llvm-cov) et web (vitest + v8).
#
# Usage :
#   ./scripts/coverage.sh              # les deux, resume au terminal
#   ./scripts/coverage.sh --rust       # Rust seulement
#   ./scripts/coverage.sh --web        # web seulement
#   ./scripts/coverage.sh --html       # produit aussi les rapports detailles
#   ./scripts/coverage.sh --seuil 40   # echoue sous ce pourcentage de lignes
#
# Equivalent de `scripts/coverage.ps1`, pour Linux, macOS et Git Bash.

set -euo pipefail

racine="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$racine"

faire_rust=false
faire_web=false
html=false
seuil=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rust) faire_rust=true; shift ;;
        --web) faire_web=true; shift ;;
        --html) html=true; shift ;;
        --seuil) seuil="$2"; shift 2 ;;
        *) echo "Option inconnue : $1" >&2; exit 2 ;;
    esac
done

# Sans option, on mesure les deux.
if [[ "$faire_rust" == false && "$faire_web" == false ]]; then
    faire_rust=true
    faire_web=true
fi

echecs=()

if [[ "$faire_rust" == true ]]; then
    echo
    echo "=== Couverture Rust ==="
    if ! cargo llvm-cov --version >/dev/null 2>&1; then
        echo "cargo-llvm-cov manquant." >&2
        echo "  Installation : rustup component add llvm-tools-preview && cargo install cargo-llvm-cov --locked" >&2
        exit 1
    fi

    # `--lib --bins` : on mesure le code du produit. Les tests d'integration
    # qui exigent PostgreSQL sont ecartes ici — sans DATABASE_URL ils echouent,
    # et un echec d'environnement ferait passer la couverture pour un probleme
    # de code. La CI, elle, a une base et les inclut.
    filtre=(--ignore-filename-regex '(tests?/|_test\.rs|/target/)')

    # Deux temps : `--no-report` execute les tests et garde les mesures brutes,
    # `report` les met en forme. En une seule commande, un test rouge fait
    # sortir cargo en erreur AVANT le rapport — et c'est justement quand ca
    # casse qu'on veut voir ce qui n'est pas couvert.
    #
    # Sans base de donnees, les tests PostgreSQL de `platform-api` echouent :
    # le rapport doit survivre a cela.
    if ! cargo llvm-cov --no-report --workspace --lib --bins --no-fail-fast; then
        echo "Des tests ont echoue : le rapport porte sur ce qui a pu s'executer." >&2
    fi

    if [[ "$html" == true ]]; then
        cargo llvm-cov report "${filtre[@]}" --html
        echo "Rapport : target/llvm-cov/html/index.html"
    fi
    cargo llvm-cov report "${filtre[@]}" --lcov --output-path target/coverage-rust.lcov >/dev/null

    # Le resume brut de llvm-cov liste TOUS les fichiers : le chiffre qui
    # interesse se perd dans plusieurs centaines de lignes. On ne garde que
    # les totaux.
    cargo llvm-cov report "${filtre[@]}" --json --summary-only 2>/dev/null         | python -c "
import json, sys
t = json.load(sys.stdin)['data'][0]['totals']
for cle, libelle in (('lines', 'Lignes'), ('functions', 'Fonctions'), ('regions', 'Regions')):
    m = t.get(cle)
    if m:
        print(f\"  {libelle:<10} {m['percent']:5.1f} %  ({m['covered']}/{m['count']})\")
" 2>/dev/null || cargo llvm-cov report "${filtre[@]}" --summary-only 2>&1 | tail -2

    if [[ "$seuil" -gt 0 ]]; then
        # Le seuil est verifie par l'outil : pas de pourcentage relu a la main,
        # donc pas de divergence entre ce qui s'affiche et ce qui est verifie.
        if ! cargo llvm-cov report "${filtre[@]}" --fail-under-lines "$seuil" --summary-only >/dev/null; then
            echecs+=("Rust sous le seuil de ${seuil} %")
        fi
    fi
fi

if [[ "$faire_web" == true ]]; then
    echo
    echo "=== Couverture web ==="
    cd "$racine/web"
    [[ -d node_modules ]] || npm ci
    npm run test:coverage || echecs+=("Suite web en echec")
    [[ "$html" == true ]] && echo "Rapport : web/coverage/index.html"
    cd "$racine"
fi

if [[ ${#echecs[@]} -gt 0 ]]; then
    echo
    echo "Echecs :" >&2
    for e in "${echecs[@]}"; do echo "  - $e" >&2; done
    exit 1
fi

echo
echo "Couverture generee."
