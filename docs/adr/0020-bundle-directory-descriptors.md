# ADR-0020: descriptor-relative bundle reads

Status: implemented; supersedes ADR-0017's concurrent-replacement limitation.

Checking a path and then opening it by name leaves a symlink replacement race.
The already locked rustix 1.1.6 exposes safe Unix open/openat, NOFOLLOW, fstatat
and descriptor-relative directory iteration. Its exact release and checksum
were verified against the official registry before adding the direct dependency.
Only its fs feature is needed; it does not enter financial library crates.

The verifier opens the root with DIRECTORY and NOFOLLOW and keeps that owned
file descriptor for every enumeration, metadata check and content read. Entry
names must be single normal components. Content opens are relative to that
root descriptor with NOFOLLOW and NONBLOCK; the opened descriptor must be a
regular file and satisfy the byte bound before reading. NONBLOCK prevents a
replacement FIFO from hanging during open. No manifest-supplied absolute or
parent path reaches a filesystem operation.

The observer is called immediately before the descriptor-relative open. Tests
replace an already checked file with an external symlink at that exact point:
it fails with E-BUNDLE-PATH before reading. A second test replaces the root path
with an external symlink; existing root descriptors still verify the original
bundle. Lexical/canonical string checks alone cannot provide this property.
The dependency contains platform unsafe internals; product crates retain their
unsafe ban. This explicit dependency decision is the third-party unsafe policy
for this boundary, not permission for new product unsafe code.
