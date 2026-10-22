# TLS trust requirements

The published `ndc-sdk` manifest constrains `rustls-webpki` to 0.103.16 or
later on the compatible 0.103 line, and the workspace lockfile is regenerated.
The direct dependency is intentional: publishing only a lockfile would not
constrain applications using this SDK. It carries the eMudhra SPKI distrust
policy used by the rustls stack.

The SDK's default reqwest feature uses native TLS, and telemetry can use
platform certificate roots. Update the deployment's OS trust independently.
Applications using webpki-roots/webpki-root-certs must also resolve 1.0.10+
(or a compatibility facade resolving to that version). Inspect the complete
application graph; older SDK releases and other rustls/webpki lines are not
fixed by upgrading a different graph edge.

Incident and patched-version reference: https://emudhra.we-have-a-problem.com/

Release validation must show certificate-verification rejection of all 12
published TLS chains (including cross-signs), plus acceptance of an unrelated
valid chain. Timeouts and connection failures are not evidence of distrust.
Rebuild consumers and update immutable deployed image references; an SDK source
change does not modify existing binaries, images, or hibernated instances.
Custom truststores, CA overrides, or custom verification require separate review.
