TERMUX_PKG_HOMEPAGE=https://github.com/donut-corp/cdomer
TERMUX_PKG_DESCRIPTION="Linguagem C-family com tipagem estatica e inferencia, transpila para C"
TERMUX_PKG_LICENSE="MIT"
TERMUX_PKG_MAINTAINER="donut-corp"
TERMUX_PKG_VERSION="0.1.0"
TERMUX_PKG_SRCURL=https://github.com/donut-corp/cdomer/archive/refs/tags/v${TERMUX_PKG_VERSION}.tar.gz
TERMUX_PKG_SHA256=7c80a5420250c77463e8adc8af43512de31e1ad8bda363b19442d7ac01bf4296
TERMUX_PKG_DEPENDS="libgcc, libc++"
TERMUX_PKG_BUILD_DEPENDS="clang"
TERMUX_PKG_NO_STATICSPLIT=true

termux_step_pre_configure() {
	termux_setup_rust
}

termux_step_make() {
	cd "${TERMUX_PKG_SRCDIR}"
	cargo build \
		--target "${CARGO_TARGET_NAME}" \
		--release \
		--offline
}

termux_step_make_install() {
	install -Dm700 \
		"${TERMUX_PKG_SRCDIR}/target/${CARGO_TARGET_NAME}/release/cdomer" \
		"${TERMUX_PREFIX}/bin/cdomer"
}
