//! Brand flag — controla qué features incluye este build.
//!
//! Para generar el build de **DigitalServer POS** (variante sin features
//! Clouget-only como el módulo Restaurante), cambiar la constante `BRAND`
//! a `Brand::DigitalServer` y recompilar.
//!
//! Las features se gatean via los métodos `tiene_modulo_*()` en este enum,
//! por lo que para excluir algo del build de DigitalServer basta con que el
//! call site verifique el flag (ej: `if branding::BRAND.tiene_modulo_restaurante()`).
//!
//! Combinado con el sistema de licencias (módulos por cliente), tenemos
//! doble capa de control:
//!   1. Brand flag (compile-time)  → qué EXISTE en el binario
//!   2. License module             → qué está ACTIVO para cada cliente

// `DigitalServer` y métodos no-restaurante no se "usan" cuando BRAND=Clouget,
// pero existen para que el día que se quiera buildear DigitalServer baste con
// cambiar la constante. No son código muerto — son alternativas compile-time.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Brand {
    Clouget,
    DigitalServer,
}

/// Marca compilada en este binario.
/// **Cambiar este valor + recompilar = build distinto.**
pub const BRAND: Brand = Brand::Clouget;

#[allow(dead_code)]
impl Brand {
    /// Nombre legible para mostrar en UI / títulos / about.
    pub const fn nombre(&self) -> &'static str {
        match self {
            Brand::Clouget => "Clouget POS",
            Brand::DigitalServer => "DigitalServer POS",
        }
    }

    /// Slug para usar en paths, URLs, identificadores.
    pub const fn slug(&self) -> &'static str {
        match self {
            Brand::Clouget => "clouget",
            Brand::DigitalServer => "digitalserver",
        }
    }

    /// Si esta marca incluye el módulo Restaurante (mesas, comandas, app móvil meseros).
    /// Solo Clouget — DigitalServer NO lo lleva.
    pub const fn tiene_modulo_restaurante(&self) -> bool {
        matches!(self, Brand::Clouget)
    }

    /// Si esta marca expone los endpoints HTTP para la app móvil de meseros.
    pub const fn tiene_app_movil_meseros(&self) -> bool {
        matches!(self, Brand::Clouget)
    }
}

/// Helper: ¿corre este binario como Clouget?
#[allow(dead_code)]
#[inline]
pub const fn es_clouget() -> bool {
    matches!(BRAND, Brand::Clouget)
}
