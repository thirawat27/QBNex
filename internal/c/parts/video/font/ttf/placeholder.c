/* Empty object built as parts/video/font/ttf/os/<os>/src.o.
 *
 * Fonts are rendered by the Rust runtime (libqb_runtime.a) since phase R7; FreeType is gone.
 * The frozen bootstrap compiler (internal/source, see L-00 in docs/rust-port/progress.md)
 * still links this object when it builds qb, so it must keep existing until stage0 is
 * regenerated. Nothing else references it.
 */
typedef int qbnex_ttf_placeholder_unused;
