use std::collections::HashMap;

pub fn get_embedded_fragments() -> HashMap<&'static str, &'static str> {
    let mut fragments = HashMap::new();

    // Base OS fragments
    fragments.insert(
        "base/debian",
        r#"FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    git \
    vim \
    && rm -rf /var/lib/apt/lists/*"#,
    );

    fragments.insert(
        "base/ubuntu",
        r#"FROM ubuntu:22.04-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    git \
    vim \
    && rm -rf /var/lib/apt/lists/*"#,
    );

    fragments.insert(
        "base/fedora",
        r#"FROM fedora:38
RUN dnf update -y && dnf install -y \
    ca-certificates \
    curl \
    git \
    vim \
    && dnf clean all"#,
    );

    // Basic feature fragments
    fragments.insert(
        "features/basic-debian",
        r#"RUN apt-get update && apt-get install -y \
    build-essential \
    sudo \
    && rm -rf /var/lib/apt/lists/*"#,
    );

    fragments.insert(
        "features/basic-ubuntu",
        r#"RUN apt-get update && apt-get install -y \
    build-essential \
    sudo \
    && rm -rf /var/lib/apt/lists/*"#,
    );

    fragments.insert(
        "features/basic-fedora",
        r#"RUN dnf install -y \
    gcc \
    gcc-c++ \
    make \
    sudo \
    && dnf clean all"#,
    );

    // Post-processing fragments
    fragments.insert(
        "post/dev-user",
        r#"RUN groupadd -r dev && useradd -r -g dev dev
RUN echo "dev ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers
USER dev
WORKDIR /home/dev
RUN mkdir -p /home/dev/.local/bin"#,
    );

    fragments.insert("post/keep-alive", r#"CMD ["sleep", "infinity"]"#);

    fragments
}

pub fn get_default_config() -> &'static str {
    r#"name = "dev-shell"

stages = [
    "@base/debian",
    "@features/basic-debian",
    "@post/dev-user",
    "@post/keep-alive",
]

docker_args = [
    "--memory=4g",
    "--cpus=2",
]

attach_command = "/bin/bash"

[[volumes]]
host = "."
container = "/workspace"
mode = "rw"
"#
}
