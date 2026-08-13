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
    "api", "web", "auth-api", "docker-agent", "ops-api", "ops-agent",
    "platform-scheduler",
    "gateway", "sentinel-bot",
    "atrium-api", "atrium-bot",
    "nexus-api", "nexus-bot",
  ]
}

group "core" {
  targets = [
    "api", "web", "auth-api", "docker-agent", "ops-api", "ops-agent",
    "platform-scheduler",
    "gateway", "sentinel-bot",
  ]
}

group "workers" {
  targets = ["platform-scheduler", "ops-agent"]
}

group "atrium" {
  targets = ["atrium-api", "atrium-bot"]
}

group "nexus" {
  targets = ["nexus-api", "nexus-bot"]
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
      "auth-api", "docker-agent", "ops-api", "ops-agent", "platform-scheduler",
      "sentinel-bot",
      "atrium-api", "atrium-bot",
      "nexus-api", "nexus-bot",
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
