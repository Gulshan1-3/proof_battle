FROM rust:latest

# Copy prebuilt Lean 4 toolchain (v4.21.0-rc3)
COPY infra/sandbox/toolchain /opt/lean
ENV PATH="/opt/lean/bin:${PATH}"

# Create non-root user proofbattle (uid 10001)
RUN groupadd -g 10001 proofbattle && \
    useradd -u 10001 -g proofbattle -m -s /bin/bash proofbattle

WORKDIR /judge

# Copy project configuration, Lake manifest, and prebuilt oleans
COPY --chown=proofbattle:proofbattle lean/lakefile.lean /judge/
COPY --chown=proofbattle:proofbattle lean/lean-toolchain /judge/
COPY --chown=proofbattle:proofbattle lean/lake-manifest.json /judge/
COPY --chown=proofbattle:proofbattle lean/.lake /judge/.lake
COPY --chown=proofbattle:proofbattle lean/Mathlib /judge/Mathlib
COPY --chown=proofbattle:proofbattle infra/sandbox/entrypoint.sh /judge/entrypoint.sh

RUN chmod +x /judge/entrypoint.sh

USER 10001:10001

ENTRYPOINT ["/judge/entrypoint.sh"]
CMD ["30", "/judge/proof.lean"]
