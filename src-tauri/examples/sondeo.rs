//! Contrasta los parsers con el /proc de esta máquina y con lo que dicen las
//! herramientas del sistema. Si un parser está mal, acá se ve.
use std::{thread, time::{Duration, Instant}};
use vasak_monitor_lib::muestreo::{cpu, discos, memoria, procesos, red};
use vasak_monitor_lib::{limpieza, registros, servicios};

fn main() {
    let a = cpu::contadores_de(&std::fs::read_to_string("/proc/stat").unwrap()).unwrap();
    let ra = red::acumulado_de(&std::fs::read_to_string("/proc/net/dev").unwrap());
    let t0 = Instant::now();
    thread::sleep(Duration::from_millis(700));
    let b = cpu::contadores_de(&std::fs::read_to_string("/proc/stat").unwrap()).unwrap();
    let rb = red::acumulado_de(&std::fs::read_to_string("/proc/net/dev").unwrap());

    println!("  CPU:     {:.1}%   ({} núcleos)", b.uso_desde(a).unwrap_or(0.0),
        cpu::nucleos_en(&std::fs::read_to_string("/proc/stat").unwrap()));

    let m = memoria::memoria_de(&std::fs::read_to_string("/proc/meminfo").unwrap());
    let (mu, uu) = discos::formato_de_tamano(m.en_uso_kib() * 1024);
    let (mt, ut) = discos::formato_de_tamano(m.total_kib * 1024);
    let (mc, uc) = discos::formato_de_tamano(m.cache_kib * 1024);
    println!("  RAM:     {:.1}% — {mu:.1} {uu} de {mt:.1} {ut} (caché {mc:.1} {uc})",
        m.uso().unwrap_or(0.0));
    match m.uso_de_swap() { Some(s) => println!("  Swap:    {s:.1}%"), None => println!("  Swap:    sin configurar") }

    if let Some(c) = rb.caudal_desde(ra, t0.elapsed()) {
        let (bj, ub) = red::formato_de_caudal(c.bajada);
        let (sb, us) = red::formato_de_caudal(c.subida);
        println!("  Red:     ↓ {bj:.1} {ub}   ↑ {sb:.1} {us}");
    }

    let montajes = discos::montajes_de(&std::fs::read_to_string("/proc/mounts").unwrap());
    println!("  Discos:  {} montajes reales", montajes.len());
    for mo in montajes.iter().take(4) { println!("           {} en {} ({})", mo.dispositivo, mo.punto, mo.tipo); }

    let mut lista = Vec::new();
    for entrada in std::fs::read_dir("/proc").unwrap().flatten() {
        let Ok(pid) = entrada.file_name().to_string_lossy().parse::<u32>() else { continue };
        let Ok(stat) = std::fs::read_to_string(entrada.path().join("stat")) else { continue };
        let Some(mut p) = procesos::proceso_de(pid, &stat) else { continue };
        let cmdline = std::fs::read(entrada.path().join("cmdline")).unwrap_or_default();
        if procesos::es_del_kernel(&p.nombre, cmdline.iter().all(|b| *b == 0)) { continue }
        if let Some(n) = procesos::nombre_de_cmdline(&cmdline) { p.nombre = n }
        lista.push(p);
    }
    let total = lista.len();
    let mut juntos = procesos::agrupar(lista);
    juntos.sort_by_key(|p| std::cmp::Reverse(p.paginas));
    let pagina = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    println!("  Apps:    {} procesos → {} aplicaciones", total, juntos.len());
    for p in juntos.iter().take(5) {
        let (v, u) = discos::formato_de_tamano(p.paginas * pagina);
        println!("           {:<24} {v:>6.1} {u}", p.nombre);
    }

    // ── Servicios, contra systemctl de verdad ────────────────────────────
    let salida = std::process::Command::new("systemctl")
        .args(["--user", "list-units", "--type=service", "--plain", "--no-legend", "--all"])
        .output().unwrap();
    let s = servicios::ordenar(servicios::servicios_de(&String::from_utf8_lossy(&salida.stdout), true));
    println!("  Servicios: {} ({} de VasakOS, {} fallidos)", s.len(),
        s.iter().filter(|x| x.de_vasakos).count(),
        s.iter().filter(|x| x.estado == "failed").count());
    for x in s.iter().take(4) { println!("           {:<34} {} / {}", x.unidad, x.estado, x.detalle); }

    // ── Registros, contra el diario de verdad ───────────────────────────
    let salida = std::process::Command::new("journalctl")
        .args(["--user", "-o", "json", "-n", "300", "--no-pager"])
        .output().unwrap();
    let e = registros::entradas_de(&String::from_utf8_lossy(&salida.stdout));
    println!("  Registros: {} entradas, {} problemas", e.len(), e.iter().filter(|x| x.es_problema()).count());
    for x in e.iter().filter(|x| x.es_problema()).take(3) {
        println!("           [{}] {}: {}", x.nivel, x.origen, x.mensaje.chars().take(60).collect::<String>());
    }

    // ── Limpieza, midiendo de verdad ────────────────────────────────────
    let home = std::env::var("HOME").unwrap();
    for t in [limpieza::Tarea::CacheDeUsuario, limpieza::Tarea::Papelera] {
        if let Some(ruta) = limpieza::ruta_de(t, &home) {
            let salida = std::process::Command::new("du").args(["-sb"]).arg(&ruta).output().unwrap();
            let bytes = limpieza::bytes_de_du(&String::from_utf8_lossy(&salida.stdout)).unwrap_or(0);
            let (v, u) = discos::formato_de_tamano(bytes);
            println!("  Limpieza:  {:<26} {v:>7.1} {u}", ruta.display().to_string());
        }
    }
    let salida = std::process::Command::new("journalctl").arg("--disk-usage").output().unwrap();
    let bytes = limpieza::bytes_del_diario(&String::from_utf8_lossy(&salida.stdout)).unwrap_or(0);
    let (v, u) = discos::formato_de_tamano(bytes);
    println!("             {:<26} {v:>7.1} {u}", "diario del sistema");
}
