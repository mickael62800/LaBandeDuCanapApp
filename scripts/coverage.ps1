# Couverture de tests du depot : Rust (cargo-llvm-cov) et web (vitest + v8).
#
# Usage :
#   .\scripts\coverage.ps1              # les deux, resume au terminal
#   .\scripts\coverage.ps1 -Rust        # Rust seulement
#   .\scripts\coverage.ps1 -Web         # web seulement
#   .\scripts\coverage.ps1 -Html        # ouvre aussi les rapports detailles
#   .\scripts\coverage.ps1 -Seuil 40    # echoue sous ce pourcentage de lignes
#
# POURQUOI PAS `cargo tarpaulin` : il instrumente mal sous Windows, et ce depot
# se developpe sous Windows. `cargo-llvm-cov` utilise l'instrumentation de LLVM,
# la meme sur les trois systemes — le chiffre local est donc celui de la CI.

[CmdletBinding()]
param(
    [switch]$Rust,
    [switch]$Web,
    [switch]$Html,
    [int]$Seuil = 0
)

$ErrorActionPreference = "Stop"
$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

# Sans option, on mesure les deux.
if (-not $Rust -and -not $Web) { $Rust = $true; $Web = $true }

function Test-Outil {
    param([string]$Commande, [string]$Installation)
    if (-not (Get-Command $Commande -ErrorAction SilentlyContinue)) {
        Write-Host "Outil manquant : $Commande" -ForegroundColor Red
        Write-Host "  Installation : $Installation"
        exit 1
    }
}

$echecs = @()

if ($Rust) {
    Write-Host "`n=== Couverture Rust ===" -ForegroundColor Cyan
    Test-Outil "cargo" "https://rustup.rs"
    if (-not (cargo llvm-cov --version 2>$null)) {
        Write-Host "cargo-llvm-cov manquant." -ForegroundColor Red
        Write-Host "  Installation : rustup component add llvm-tools-preview; cargo install cargo-llvm-cov --locked"
        exit 1
    }

    # `--lib --bins` : on mesure le code du produit. Les tests d'integration
    # qui exigent PostgreSQL sont exclus ici — sans DATABASE_URL ils echouent,
    # et un echec d'environnement ferait passer la couverture pour un probleme
    # de code. La CI, elle, a une base et les inclut.
    #
    # `--no-fail-fast` : un test rouge ne doit pas priver du rapport ; c'est
    # souvent quand ca casse qu'on veut voir ce qui n'est pas couvert.
    $filtre = @("--ignore-filename-regex", "(tests?/|_test\.rs|/target/)")

    # Deux temps : `--no-report` execute les tests et garde les mesures brutes,
    # `report` les met en forme. En une seule commande, un test rouge fait
    # sortir cargo en erreur AVANT le rapport — et c'est justement quand ca
    # casse qu'on veut voir ce qui n'est pas couvert.
    #
    # Sans base de donnees, les tests PostgreSQL de `platform-api` echouent :
    # le rapport doit survivre a cela.
    cargo llvm-cov --no-report --workspace --lib --bins --no-fail-fast
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Des tests ont echoue : le rapport porte sur ce qui a pu s'executer." -ForegroundColor Yellow
    }

    if ($Html) {
        cargo llvm-cov report @filtre --html
        Write-Host "Rapport : target\llvm-cov\html\index.html"
    }
    cargo llvm-cov report @filtre --lcov --output-path target/coverage-rust.lcov | Out-Null

    # Le resume brut de llvm-cov liste TOUS les fichiers : le chiffre qui
    # interesse se perd dans plusieurs centaines de lignes. On ne garde que
    # les totaux.
    $json = cargo llvm-cov report @filtre --json --summary-only 2>$null
    if ($LASTEXITCODE -eq 0 -and $json) {
        $totaux = ($json | ConvertFrom-Json).data[0].totals
        foreach ($mesure in @(
            @{ Cle = "lines"; Libelle = "Lignes" },
            @{ Cle = "functions"; Libelle = "Fonctions" },
            @{ Cle = "regions"; Libelle = "Regions" }
        )) {
            $m = $totaux.($mesure.Cle)
            if ($m) {
                Write-Host ("  {0,-10} {1,5:N1} %  ({2}/{3})" -f $mesure.Libelle, $m.percent, $m.covered, $m.count)
            }
        }
    } else {
        cargo llvm-cov report @filtre --summary-only 2>&1 | Select-Object -Last 2 | ForEach-Object { Write-Host $_ }
    }

    if ($Seuil -gt 0) {
        # `--fail-under-lines` fait le travail cote outil : pas de pourcentage
        # relu a la main, donc pas de divergence entre ce qui est affiche et ce
        # qui est verifie.
        cargo llvm-cov report @filtre --fail-under-lines $Seuil --summary-only | Out-Null
        if ($LASTEXITCODE -ne 0) { $echecs += "Rust sous le seuil de $Seuil %" }
    }
}

if ($Web) {
    Write-Host "`n=== Couverture web ===" -ForegroundColor Cyan
    Test-Outil "npm" "https://nodejs.org"
    Push-Location (Join-Path $racine "web")
    try {
        if (-not (Test-Path "node_modules")) { npm ci }
        npm run test:coverage
        if ($LASTEXITCODE -ne 0) { $echecs += "Suite web en echec" }
        if ($Html) {
            $rapport = Join-Path (Get-Location) "coverage\index.html"
            if (Test-Path $rapport) {
                Write-Host "Rapport : $rapport"
                Start-Process $rapport
            }
        }
    } finally {
        Pop-Location
    }
}

if ($echecs.Count -gt 0) {
    Write-Host "`nEchecs :" -ForegroundColor Red
    $echecs | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host "`nCouverture generee." -ForegroundColor Green
