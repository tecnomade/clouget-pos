use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Producto {
    pub id: Option<i64>,
    pub codigo: Option<String>,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub categoria_id: Option<i64>,
    pub precio_costo: f64,
    pub precio_venta: f64,
    pub iva_porcentaje: f64,
    pub incluye_iva: bool,
    pub stock_actual: f64,
    pub stock_minimo: f64,
    pub unidad_medida: String,
    pub es_servicio: bool,
    pub activo: bool,
    pub imagen: Option<String>,
    #[serde(default)]
    pub requiere_serie: bool,
    #[serde(default)]
    pub requiere_caducidad: bool,
    #[serde(default)]
    pub no_controla_stock: bool,
    /// 'SIMPLE' (default) | 'COMBO_FIJO' | 'COMBO_FLEXIBLE'
    #[serde(default = "default_tipo_producto")]
    pub tipo_producto: String,
    /// 'COCINA' (default) | 'BARRA' | 'DIRECTO'
    /// Restaurante: define a dónde va el item al ser agregado a un pedido.
    /// - COCINA: el cocinero lo prepara (aparece en /cocina)
    /// - BARRA:  el bartender lo prepara (también en /cocina, distinguible por código)
    /// - DIRECTO: el mesero lo despacha sin preparación (bebidas embotelladas, snacks)
    #[serde(default = "default_destino_preparacion")]
    pub destino_preparacion: String,
}

fn default_tipo_producto() -> String { "SIMPLE".to_string() }
fn default_destino_preparacion() -> String { "COCINA".to_string() }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComboGrupo {
    pub id: Option<i64>,
    pub producto_padre_id: i64,
    pub nombre: String,
    pub minimo: i64,
    pub maximo: i64,
    #[serde(default)]
    pub orden: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComboComponente {
    pub id: Option<i64>,
    pub producto_padre_id: i64,
    pub producto_hijo_id: i64,
    pub cantidad: f64,
    #[serde(default)]
    pub grupo_id: Option<i64>,
    #[serde(default)]
    pub orden: i64,
    /// v2.5.89: precio que esta opción suma al precio base del combo (extras).
    #[serde(default)]
    pub precio_extra: f64,
    /// v2.5.89: etiqueta opcional para distinguir opciones del mismo ingrediente
    /// (ej: "Tipo 1 (2 alitas)" vs "Tipo 2 (6 alitas)"). Si vacío, se usa el nombre del hijo.
    #[serde(default)]
    pub etiqueta: Option<String>,
    // Campos enriquecidos en lecturas (no se serializan al insert):
    #[serde(default)]
    pub hijo_nombre: Option<String>,
    #[serde(default)]
    pub hijo_codigo: Option<String>,
    #[serde(default)]
    pub hijo_precio_venta: Option<f64>,
    #[serde(default)]
    pub hijo_precio_costo: Option<f64>,
    #[serde(default)]
    pub hijo_stock_actual: Option<f64>,
    #[serde(default)]
    pub hijo_unidad_medida: Option<String>,
    #[serde(default)]
    pub hijo_no_controla_stock: Option<bool>,
    #[serde(default)]
    pub hijo_es_servicio: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoTactil {
    pub id: i64,
    pub nombre: String,
    pub precio_venta: f64,
    pub iva_porcentaje: f64,
    pub incluye_iva: bool,
    pub stock_actual: f64,
    pub categoria_id: Option<i64>,
    pub categoria_nombre: Option<String>,
    pub imagen: Option<String>,
    pub es_servicio: bool,
    pub no_controla_stock: bool,
    /// 'SIMPLE' | 'COMBO_FIJO' | 'COMBO_FLEXIBLE'
    #[serde(default = "default_tipo_producto")]
    pub tipo_producto: String,
    /// Stock calculado para COMBO_FIJO (MIN de stock_hijo/cant). None para SIMPLE y COMBO_FLEXIBLE.
    #[serde(default)]
    pub stock_combo: Option<f64>,
    /// Descripcion / info adicional del producto (para busqueda en POS)
    #[serde(default)]
    pub descripcion: Option<String>,
    /// Codigo del producto (para busqueda en POS)
    #[serde(default)]
    pub codigo: Option<String>,
    /// Codigo de barras (para busqueda en POS)
    #[serde(default)]
    pub codigo_barras: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductoBusqueda {
    pub id: i64,
    pub codigo: Option<String>,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub precio_venta: f64,
    #[serde(default)]
    pub precio_costo: f64,
    pub iva_porcentaje: f64,
    pub incluye_iva: bool,
    pub stock_actual: f64,
    pub stock_minimo: f64,
    pub categoria_nombre: Option<String>,
    pub precio_lista: Option<f64>,
    /// v2.4.14: flag indicador (no la imagen completa) para mostrar miniatura en listado.
    /// El frontend hace lazy-load de la imagen real solo cuando entra al viewport.
    #[serde(default)]
    pub tiene_imagen: bool,
    /// v2.5.21: flags para que el frontend pueda excluir servicios y productos sin
    /// control de stock del cálculo de stock disponible en combos.
    #[serde(default)]
    pub es_servicio: bool,
    #[serde(default)]
    pub no_controla_stock: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Categoria {
    pub id: Option<i64>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activo: bool,
}
