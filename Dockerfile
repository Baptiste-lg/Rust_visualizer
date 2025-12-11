# Étape 1 : Construction (Builder)
# On utilise l'image officielle Rust pour compiler
FROM rust:1.83 as builder

WORKDIR /usr/src/app
COPY . .

# On compile en mode release pour avoir les perfs maximales
RUN cargo build --release

# Étape 2 : Image finale (Runtime)
# On utilise une image minimale sécurisée (Google Distroless)
# Cela rend ton image très légère (quelques Mo au lieu de 1Go+)
FROM gcr.io/distroless/cc-debian12

# On copie uniquement l'exécutable compilé depuis l'étape 1
# Remplace "Rust_visualizer" par le nom exact de ton binaire dans Cargo.toml si différent
COPY --from=builder /usr/src/app/target/release/Rust_visualizer /app/rust-visualizer

# Commande de lancement
ENTRYPOINT ["/app/rust-visualizer"]