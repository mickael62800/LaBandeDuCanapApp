# ============================================================================
# BuildKit Bake — construit en parallele les images reellement deployees par
# docker-compose.yml. Lancer depuis la racine du depot :
#
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl core
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl workers
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl atrium nexus
# ============================================================================

variable "TAG" { default = "latest" }

group "default" {
  targets = [
    "api", "web", "auth-api", "docker-agent", "ops-api", "ops-worker",
    "gateway", "sentinel-bot", "sentinel-worker",
    "atrium-api", "atrium-bot", "atrium-worker",
    "nexus-api", "nexus-bot", "nexus-worker",
  ]
}

group "core" {
  targets = [
    "api", "web", "auth-api", "docker-agent", "ops-api", "ops-worker",
    "gateway", "sentinel-bot", "sentinel-worker",
  ]
}

group "workers" {
  targets = ["sentinel-worker", "ops-worker", "atrium-worker", "nexus-worker"]
}

group "atrium" {
  targets = ["atrium-api", "atrium-bot", "atrium-worker"]
}

group "nexus" {
  targets = ["nexus-api", "nexus-bot", "nexus-worker"]
}

target "_alpine-base" {
  context    = "."
  dockerfile = "infrastructure/docker/Dockerfile.rust-alpine"
}

target "_debian-base" {
  context    = "."
  dockerfile = "infrastructure/docker/Dockerfile.rust-debian"
}

target "api" {
  inherits = ["_debian-base"]
  args = {
    BIN_NAME       = "sentinel-api"
    MIGRATIONS_SRC = "sentinel-api/migrations"
  }
  tags = ["discordsentinel-api:${TAG}"]
}

target "gateway" {
  inherits = ["_alpine-base"]
  args     = { BIN_NAME = "sentinel-gateway" }
  tags     = ["discordsentinel-gateway:${TAG}"]
}

# Toutes les autres applications Rust partagent le meme Dockerfile Alpine.
# La matrice maintient un seul cache Cargo Chef/BuildKit par dependances tout
# en produisant une cible et une image distinctes par binaire.
target "rust-app" {
  inherits = ["_alpine-base"]
  matrix = {
    app = [
      "auth-api", "docker-agent", "ops-api", "ops-worker",
      "sentinel-bot", "sentinel-worker",
      "atrium-api", "atrium-bot", "atrium-worker",
      "nexus-api", "nexus-bot", "nexus-worker",
    ]
  }
  name = "${app}"
  args = { BIN_NAME = "${app}" }
  tags = ["discordsentinel-${app}:${TAG}"]
}

target "web" {
  context    = "."
  dockerfile = "web/Dockerfile"
  tags       = ["discordsentinel-web:${TAG}"]
}
