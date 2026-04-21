use crate::db::{Database, SesionState};
use crate::models::{NuevoUsuario, SesionActiva, UsuarioInfo};
use crate::utils;
use tauri::State;

/// Verifica el PIN o contraseña contra todos los usuarios activos.
/// Si coincide, establece la sesión activa.
/// El modo de autenticación se determina por la config 'modo_login':
///   'pin' => solo PIN, 'password' => solo contraseña, 'ambos' => PIN o contraseña
#[tauri::command]
pub fn iniciar_sesion(
    db: State<Database>,
    sesion: State<SesionState>,
    pin: String,
    password: Option<String>,
) -> Result<SesionActiva, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Leer modo_login de config
    let modo_login: String = conn
        .query_row(
            "SELECT value FROM config WHERE key = 'modo_login'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "pin".to_string());

    let mut stmt = conn
        .prepare("SELECT id, nombre, pin_hash, pin_salt, rol, permisos, password_hash, password_salt FROM usuarios WHERE activo = 1")
        .map_err(|e| e.to_string())?;

    let usuarios: Vec<(i64, String, String, String, String, String, Option<String>, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Intentar autenticación por PIN (si modo es 'pin' o 'ambos')
    if modo_login == "pin" || modo_login == "ambos" {
        if !pin.is_empty() {
            for (id, nombre, pin_hash, pin_salt, rol, permisos, _, _) in &usuarios {
                let hash_intento = utils::hash_pin(pin_salt, &pin);
                if hash_intento == *pin_hash {
                    let nueva_sesion = SesionActiva {
                        usuario_id: *id,
                        nombre: nombre.clone(),
                        rol: rol.clone(),
                        permisos: permisos.clone(),
                    };
                    let mut sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
                    *sesion_guard = Some(nueva_sesion.clone());
                    // Persistir sesión en config para sobrevivir reinicios
                    if let Ok(json) = serde_json::to_string(&nueva_sesion) {
                        let _ = conn.execute(
                            "INSERT INTO config (key, value) VALUES ('sesion_activa', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
                            rusqlite::params![json],
                        );
                    }
                    return Ok(nueva_sesion);
                }
            }
            // En modo 'pin' puro, retornar error inmediatamente
            if modo_login == "pin" {
                return Err("PIN incorrecto".to_string());
            }
        }
    }

    // Intentar autenticación por contraseña (si se envió password, siempre intentar)
    if password.is_some() {
        if let Some(ref pwd) = password {
            if !pwd.is_empty() {
                for (id, nombre, _, _, rol, permisos, password_hash, password_salt) in &usuarios {
                    if let (Some(pw_hash), Some(pw_salt)) = (password_hash, password_salt) {
                        let hash_intento = utils::hash_pin(pw_salt, pwd);
                        if hash_intento == *pw_hash {
                            let nueva_sesion = SesionActiva {
                                usuario_id: *id,
                                nombre: nombre.clone(),
                                rol: rol.clone(),
                                permisos: permisos.clone(),
                            };
                            let mut sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
                            *sesion_guard = Some(nueva_sesion.clone());
                            // Persistir sesión en config para sobrevivir reinicios
                            if let Ok(json) = serde_json::to_string(&nueva_sesion) {
                                let _ = conn.execute(
                                    "INSERT INTO config (key, value) VALUES ('sesion_activa', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
                                    rusqlite::params![json],
                                );
                            }
                            return Ok(nueva_sesion);
                        }
                    }
                }
                return Err("Contraseña incorrecta. Verifique que el usuario tiene contraseña configurada en Configuracion.".to_string());
            }
        }
    }

    Err("Credenciales incorrectas".to_string())
}

/// Cierra la sesión activa
#[tauri::command]
pub fn cerrar_sesion(db: State<Database>, sesion: State<SesionState>) -> Result<(), String> {
    let mut sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    *sesion_guard = None;
    // Limpiar sesión persistida
    if let Ok(conn) = db.conn.lock() {
        let _ = conn.execute("DELETE FROM config WHERE key = 'sesion_activa'", []);
    }
    Ok(())
}

/// Retorna la sesión activa (o null si no hay).
/// Si la memoria está vacía, intenta restaurar desde config (persistencia).
#[tauri::command]
pub fn obtener_sesion_actual(
    db: State<Database>,
    sesion: State<SesionState>,
) -> Result<Option<SesionActiva>, String> {
    let mut sesion_guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    if sesion_guard.is_some() {
        return Ok(sesion_guard.clone());
    }
    // Intentar restaurar desde config
    if let Ok(conn) = db.conn.lock() {
        let json: Result<String, _> = conn.query_row(
            "SELECT value FROM config WHERE key = 'sesion_activa'",
            [],
            |row| row.get(0),
        );
        if let Ok(json_str) = json {
            if let Ok(restaurada) = serde_json::from_str::<SesionActiva>(&json_str) {
                *sesion_guard = Some(restaurada.clone());
                return Ok(Some(restaurada));
            }
        }
    }
    Ok(None)
}

/// Crea un nuevo usuario. Requiere sesión ADMIN.
#[tauri::command]
pub fn crear_usuario(
    db: State<Database>,
    sesion: State<SesionState>,
    usuario: NuevoUsuario,
) -> Result<UsuarioInfo, String> {
    // Verificar que la sesión sea ADMIN
    verificar_admin(&sesion)?;

    // Validar PIN: solo 4-6 dígitos
    if !usuario.pin.chars().all(|c| c.is_ascii_digit()) || usuario.pin.len() < 4 || usuario.pin.len() > 6
    {
        return Err("El PIN debe tener 4 a 6 dígitos numéricos".to_string());
    }

    // Validar rol
    if usuario.rol != "ADMIN" && usuario.rol != "CAJERO" && usuario.rol != "TECNICO" {
        return Err("El rol debe ser ADMIN, CAJERO o TECNICO".to_string());
    }

    // Validar nombre no vacío
    let nombre = usuario.nombre.trim().to_uppercase();
    if nombre.is_empty() {
        return Err("El nombre no puede estar vacío".to_string());
    }

    // Validar permisos JSON si se proporcionan
    let permisos = usuario.permisos.unwrap_or_else(|| "{}".to_string());
    // Verificar que sea JSON válido
    serde_json::from_str::<serde_json::Value>(&permisos)
        .map_err(|_| "El campo permisos debe ser un JSON válido".to_string())?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar nombre único
    let existe: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE nombre = ?1",
            rusqlite::params![nombre],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if existe {
        return Err(format!("Ya existe un usuario con el nombre '{}'", nombre));
    }

    let salt = utils::generar_salt();
    let pin_hash = utils::hash_pin(&salt, &usuario.pin);

    conn.execute(
        "INSERT INTO usuarios (nombre, pin_hash, pin_salt, rol, activo, permisos)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        rusqlite::params![nombre, pin_hash, salt, usuario.rol, permisos],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(UsuarioInfo {
        id,
        nombre,
        rol: usuario.rol,
        activo: true,
        permisos,
    })
}

/// Lista todos los usuarios (sin hash/salt). Requiere ADMIN.
#[tauri::command]
pub fn listar_usuarios(
    db: State<Database>,
    sesion: State<SesionState>,
) -> Result<Vec<UsuarioInfo>, String> {
    verificar_admin(&sesion)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, nombre, rol, activo, permisos FROM usuarios ORDER BY id")
        .map_err(|e| e.to_string())?;

    let usuarios = stmt
        .query_map([], |row| {
            Ok(UsuarioInfo {
                id: row.get(0)?,
                nombre: row.get(1)?,
                rol: row.get(2)?,
                activo: row.get::<_, i64>(3).map(|v| v == 1)?,
                permisos: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(usuarios)
}

/// Actualiza un usuario. Requiere ADMIN.
#[tauri::command]
pub fn actualizar_usuario(
    db: State<Database>,
    sesion: State<SesionState>,
    id: i64,
    nombre: Option<String>,
    pin: Option<String>,
    rol: Option<String>,
    activo: Option<bool>,
    permisos: Option<String>,
) -> Result<UsuarioInfo, String> {
    verificar_admin(&sesion)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que el usuario existe
    let (_current_nombre, current_rol, current_activo): (String, String, bool) = conn
        .query_row(
            "SELECT nombre, rol, activo FROM usuarios WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2).map(|v| v == 1)?)),
        )
        .map_err(|_| "Usuario no encontrado".to_string())?;

    let new_rol = rol.as_deref().unwrap_or(&current_rol);
    let new_activo = activo.unwrap_or(current_activo);

    // Proteger: no desactivar ni cambiar rol del último admin activo
    if current_rol == "ADMIN" && (new_rol != "ADMIN" || !new_activo) {
        let admin_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usuarios WHERE rol = 'ADMIN' AND activo = 1 AND id != ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if admin_count == 0 {
            return Err("No se puede desactivar o cambiar el rol del último administrador activo".to_string());
        }
    }

    // Actualizar nombre
    if let Some(ref new_nombre) = nombre {
        let n = new_nombre.trim().to_uppercase();
        if n.is_empty() {
            return Err("El nombre no puede estar vacío".to_string());
        }
        // Verificar unicidad
        let existe: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM usuarios WHERE nombre = ?1 AND id != ?2",
                rusqlite::params![n, id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);
        if existe {
            return Err(format!("Ya existe un usuario con el nombre '{}'", n));
        }
        conn.execute(
            "UPDATE usuarios SET nombre = ?1 WHERE id = ?2",
            rusqlite::params![n, id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Actualizar PIN
    if let Some(ref new_pin) = pin {
        if !new_pin.chars().all(|c| c.is_ascii_digit()) || new_pin.len() < 4 || new_pin.len() > 6 {
            return Err("El PIN debe tener 4 a 6 dígitos numéricos".to_string());
        }
        let salt = utils::generar_salt();
        let pin_hash = utils::hash_pin(&salt, new_pin);
        conn.execute(
            "UPDATE usuarios SET pin_hash = ?1, pin_salt = ?2 WHERE id = ?3",
            rusqlite::params![pin_hash, salt, id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Actualizar rol
    if let Some(ref new_rol_str) = rol {
        if new_rol_str != "ADMIN" && new_rol_str != "CAJERO" && new_rol_str != "TECNICO" {
            return Err("El rol debe ser ADMIN, CAJERO o TECNICO".to_string());
        }
        conn.execute(
            "UPDATE usuarios SET rol = ?1 WHERE id = ?2",
            rusqlite::params![new_rol_str, id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Actualizar activo
    if let Some(new_activo_val) = activo {
        conn.execute(
            "UPDATE usuarios SET activo = ?1 WHERE id = ?2",
            rusqlite::params![new_activo_val as i64, id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Actualizar permisos
    if let Some(ref new_permisos) = permisos {
        // Verificar que sea JSON válido
        serde_json::from_str::<serde_json::Value>(new_permisos)
            .map_err(|_| "El campo permisos debe ser un JSON válido".to_string())?;
        conn.execute(
            "UPDATE usuarios SET permisos = ?1 WHERE id = ?2",
            rusqlite::params![new_permisos, id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Retornar usuario actualizado
    let updated = conn
        .query_row(
            "SELECT id, nombre, rol, activo, permisos FROM usuarios WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(UsuarioInfo {
                    id: row.get(0)?,
                    nombre: row.get(1)?,
                    rol: row.get(2)?,
                    activo: row.get::<_, i64>(3).map(|v| v == 1)?,
                    permisos: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(updated)
}

/// Desactiva (soft-delete) un usuario. Requiere ADMIN.
#[tauri::command]
pub fn eliminar_usuario(
    db: State<Database>,
    sesion: State<SesionState>,
    id: i64,
) -> Result<(), String> {
    verificar_admin(&sesion)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que el usuario existe y su rol
    let rol: String = conn
        .query_row(
            "SELECT rol FROM usuarios WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|_| "Usuario no encontrado".to_string())?;

    // Proteger último admin
    if rol == "ADMIN" {
        let admin_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM usuarios WHERE rol = 'ADMIN' AND activo = 1 AND id != ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if admin_count == 0 {
            return Err("No se puede eliminar el último administrador activo".to_string());
        }
    }

    conn.execute(
        "UPDATE usuarios SET activo = 0 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Verifica un PIN de administrador sin cambiar la sesión activa.
/// Retorna el nombre del admin si el PIN es correcto.
#[tauri::command]
pub fn verificar_pin_admin(db: State<Database>, pin: String) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, nombre, pin_hash, pin_salt, rol FROM usuarios WHERE activo = 1 AND rol = 'ADMIN'")
        .map_err(|e| e.to_string())?;

    let admins: Vec<(i64, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for (_id, nombre, pin_hash, pin_salt) in admins {
        let hash_intento = utils::hash_pin(&pin_salt, &pin);
        if hash_intento == pin_hash {
            return Ok(nombre);
        }
    }

    Err("PIN de administrador incorrecto".to_string())
}

/// Retorna la lista de permisos disponibles en el sistema.
#[tauri::command]
pub fn obtener_permisos_disponibles() -> Vec<(String, String)> {
    use crate::models::usuario::PERMISOS_DISPONIBLES;
    PERMISOS_DISPONIBLES
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Lista nombres de usuarios activos (sin autenticación requerida).
/// Solo retorna id y nombre para el selector de login.
#[tauri::command]
pub fn listar_usuarios_login(
    db: State<Database>,
) -> Result<Vec<(i64, String)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, nombre FROM usuarios WHERE activo = 1 ORDER BY nombre")
        .map_err(|e| e.to_string())?;

    let usuarios = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(usuarios)
}

/// Establece o cambia la contraseña de un usuario. Requiere ADMIN.
#[tauri::command]
pub fn cambiar_password(
    db: State<Database>,
    sesion: State<SesionState>,
    usuario_id: i64,
    password: String,
) -> Result<(), String> {
    verificar_admin(&sesion)?;

    if password.len() < 6 {
        return Err("La contraseña debe tener al menos 6 caracteres".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Verificar que el usuario existe
    let _: String = conn
        .query_row(
            "SELECT nombre FROM usuarios WHERE id = ?1",
            rusqlite::params![usuario_id],
            |row| row.get(0),
        )
        .map_err(|_| "Usuario no encontrado".to_string())?;

    let salt = utils::generar_salt();
    let password_hash = utils::hash_pin(&salt, &password);

    conn.execute(
        "UPDATE usuarios SET password_hash = ?1, password_salt = ?2 WHERE id = ?3",
        rusqlite::params![password_hash, salt, usuario_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Helper: verifica que la sesión actual sea ADMIN
pub(crate) fn verificar_admin(sesion: &State<SesionState>) -> Result<(), String> {
    let guard = sesion.sesion.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(s) if s.rol == "ADMIN" => Ok(()),
        Some(_) => Err("Se requiere permisos de administrador".to_string()),
        None => Err("Debe iniciar sesión".to_string()),
    }
}
