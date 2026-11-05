//! Textos de la interfaz en los cuatro idiomas, buscados por el texto en inglés.

use crate::i18n::Lang;

/// (inglés, español, portugués, chino)
const STRINGS: &[(&str, &str, &str, &str)] = &[
    (
        "Sandbox ready. Work in the console, then press F5 to check.",
        "Sandbox listo. Trabajá en la consola y apretá F5 para verificar.",
        "Sandbox pronto. Trabalhe no console e aperte F5 para verificar.",
        "沙盒已就绪。在控制台中操作，然后按 F5 检查。",
    ),
    (
        "✓ Correct! Press F6 to continue.",
        "✓ ¡Correcto! Apretá F6 para continuar.",
        "✓ Correto! Aperte F6 para continuar.",
        "✓ 正确！按 F6 继续。",
    ),
    ("points", "puntos", "pontos", "分"),
    (
        "(sandbox restored to the start of the exercise)",
        "(el sandbox volvió al inicio del ejercicio)",
        "(o sandbox voltou ao início do exercício)",
        "（沙盒已恢复到练习开始时的状态）",
    ),
    (
        "Achievement unlocked",
        "Logro desbloqueado",
        "Conquista desbloqueada",
        "解锁成就",
    ),
    (
        "Take the placement exam",
        "Rendir el examen de ubicación",
        "Fazer o exame de nivelamento",
        "参加分级测试",
    ),
    (
        "Skip it and start at level 1",
        "Omitirlo y empezar en el nivel 1",
        "Pular e começar no nível 1",
        "跳过，从第 1 级开始",
    ),
    (
        "Earlier levels always stay available.  ↑/↓ · Enter · L language",
        "Los niveles anteriores siempre quedan disponibles.  ↑/↓ · Enter · L idioma",
        "Os níveis anteriores sempre ficam disponíveis.  ↑/↓ · Enter · L idioma",
        "之前的级别始终可用。  ↑/↓ · Enter · L 语言",
    ),
    (
        " Placement exam ",
        " Examen de ubicación ",
        " Exame de nivelamento ",
        " 分级测试 ",
    ),
    (
        "Exam finished.",
        "Examen terminado.",
        "Exame concluído.",
        "测试完成。",
    ),
    (
        "You start at level",
        "Arrancás en el nivel",
        "Você começa no nível",
        "你从这一级开始：",
    ),
    (
        "Enter: continue",
        "Enter: continuar",
        "Enter: continuar",
        "Enter：继续",
    ),
    ("Question", "Pregunta", "Pergunta", "问题"),
    (
        "↑/↓ or 1-4 to choose · Enter to answer",
        "↑/↓ o 1-4 para elegir · Enter para responder",
        "↑/↓ ou 1-4 para escolher · Enter para responder",
        "↑/↓ 或 1-4 选择 · Enter 回答",
    ),
    (
        "  ← your start",
        "  ← tu inicio",
        "  ← seu início",
        "  ← 你的起点",
    ),
    ("Level", "Nivel", "Nível", "级别"),
    ("Free mode", "Modo libre", "Modo livre", "自由模式"),
    ("Achievements", "Logros", "Conquistas", "成就"),
    (
        "↑/↓ move · Enter open · L: English/Español · Q quit",
        "↑/↓ mover · Enter abrir · L: English/Español · Q salir",
        "↑/↓ mover · Enter abrir · L: idioma · Q sair",
        "↑/↓ 移动 · Enter 打开 · L：语言 · Q 退出",
    ),
    ("Coming soon.", "Próximamente.", "Em breve.", "敬请期待。"),
    (
        "↑/↓ move · Enter open lesson · L language · Esc back",
        "↑/↓ mover · Enter abrir lección · L idioma · Esc volver",
        "↑/↓ mover · Enter abrir lição · L idioma · Esc voltar",
        "↑/↓ 移动 · Enter 打开课程 · L 语言 · Esc 返回",
    ),
    ("Esc: back", "Esc: volver", "Esc: voltar", "Esc：返回"),
    (
        "This topic is explained without an exercise.",
        "Este tema se explica sin ejercicio.",
        "Este tema é explicado sem exercício.",
        "这个主题只讲解，没有练习。",
    ),
    (
        "↑/↓ scroll · Enter start exercises · L language · Esc back",
        "↑/↓ desplazar · Enter empezar ejercicios · L idioma · Esc volver",
        "↑/↓ rolar · Enter começar os exercícios · L idioma · Esc voltar",
        "↑/↓ 滚动 · Enter 开始练习 · L 语言 · Esc 返回",
    ),
    (
        "↑/↓ scroll · L language · Esc back",
        "↑/↓ desplazar · L idioma · Esc volver",
        "↑/↓ rolar · L idioma · Esc voltar",
        "↑/↓ 滚动 · L 语言 · Esc 返回",
    ),
    ("Hint", "Pista", "Dica", "提示"),
    ("✓ Correct!", "✓ ¡Correcto!", "✓ Correto!", "✓ 正确！"),
    ("exercise", "ejercicio", "exercício", "练习"),
    (" Console ", " Consola ", " Console ", " 控制台 "),
    (
        "F6: continue · Esc: back to the menu",
        "F6: continuar · Esc: volver al menú",
        "F6: continuar · Esc: voltar ao menu",
        "F6：继续 · Esc：返回菜单",
    ),
    (
        "F5: check · F1: hint · Ctrl+R: restart · Esc: menu · Ctrl+Q: quit",
        "F5: verificar · F1: pista · Ctrl+R: reiniciar · Esc: menú · Ctrl+Q: salir",
        "F5: verificar · F1: dica · Ctrl+R: reiniciar · Esc: menu · Ctrl+Q: sair",
        "F5：检查 · F1：提示 · Ctrl+R：重新开始 · Esc：菜单 · Ctrl+Q：退出",
    ),
    (" Free mode ", " Modo libre ", " Modo livre ", " 自由模式 "),
    (
        "Free mode: your own SQLite database, saved between sessions. Nothing outside the sandbox is ever touched.",
        "Modo libre: tu propia base SQLite, que se guarda entre sesiones. Nada fuera del sandbox se toca jamás.",
        "Modo livre: seu próprio banco SQLite, salvo entre sessões. Nada fora do sandbox é tocado.",
        "自由模式：你自己的 SQLite 数据库，会在多次使用之间保存。沙盒之外的任何内容都不会被改动。",
    ),
    (
        "Type .help for the commands. Statements end with ;",
        "Escribí .help para ver los comandos. Las sentencias terminan con ;",
        "Digite .help para ver os comandos. As instruções terminam com ;",
        "输入 .help 查看命令。语句以 ; 结尾",
    ),
    (
        "Learn SQL by practicing real SQL, inside your terminal",
        "Aprendé SQL practicando con SQL real, dentro de tu terminal",
        "Aprenda SQL praticando com SQL de verdade, dentro do seu terminal",
        "在终端里用真实的 SQL 练习，学会 SQL",
    ),
    (
        "-A sandbox: embedded SQLite, nothing outside it is ever touched-",
        "-Un sandbox: SQLite embebido, nada fuera de él se toca-",
        "-Um sandbox: SQLite embutido, nada fora dele é tocado-",
        "-沙盒练习：内置 SQLite，不会改动沙盒之外的任何内容-",
    ),
    (
        "Do you already know some SQL? A 10-question placement exam picks your starting level.",
        "¿Ya sabés algo de SQL? Un examen de ubicación de 10 preguntas elige tu nivel de arranque.",
        "Você já sabe algo de SQL? Um exame de nivelamento com 10 perguntas escolhe seu nível inicial.",
        "你已经懂一些 SQL 吗？一个包含 10 道题的分级测试会决定你的起始级别。",
    ),
    ("What now?", "¿Y ahora?", "E agora?", "接下来做什么？"),
    (
        "You finished every practice exercise. Well done!",
        "¡Terminaste todos los ejercicios prácticos! Muy bien.",
        "Você terminou todos os exercícios práticos! Muito bem.",
        "你完成了所有练习！太棒了。",
    ),
    (
        "To keep practicing on real databases, try lazysql: a terminal UI for databases, the same idea as lazygit is for git.",
        "Para seguir practicando con bases de datos reales, probá lazysql: una interfaz de terminal para bases de datos, la misma idea que lazygit para git.",
        "Para continuar praticando com bancos de dados reais, experimente o lazysql: uma interface de terminal para bancos de dados, a mesma ideia do lazygit para o git.",
        "想继续在真实数据库上练习，可以试试 lazysql：一个数据库的终端界面，思路和给 git 用的 lazygit 一样。",
    ),
    (
        "Install it (pick one):",
        "Instalalo (elegí una opción):",
        "Instale (escolha uma opção):",
        "安装方式（任选其一）：",
    ),
    (
        "    Or download a binary for Windows, macOS or Linux from github.com/jorgerojas26/lazysql/releases",
        "    O bajá un binario para Windows, macOS o Linux de github.com/jorgerojas26/lazysql/releases",
        "    Ou baixe um binário para Windows, macOS ou Linux em github.com/jorgerojas26/lazysql/releases",
        "    或从 github.com/jorgerojas26/lazysql/releases 下载 Windows、macOS 或 Linux 的可执行文件",
    ),
    (
        "Open your free-mode database with it:",
        "Abrí con él la base de datos de tu modo libre:",
        "Abra com ele o banco de dados do seu modo livre:",
        "用它打开你自由模式的数据库：",
    ),
    (
        "    Everything you create in Free mode is saved in that file between sessions.",
        "    Todo lo que creás en el modo libre se guarda en ese archivo entre sesiones.",
        "    Tudo o que você cria no modo livre fica salvo nesse arquivo entre sessões.",
        "    你在自由模式里创建的一切都会保存在这个文件中。",
    ),
    (
        "Running… Ctrl+C: stop the query",
        "Ejecutando… Ctrl+C: detener la consulta",
        "Executando… Ctrl+C: parar a consulta",
        "执行中… Ctrl+C：停止查询",
    ),
    (
        "Erase the free-mode database? Everything you created will be lost.  y: yes · any other key: no",
        "¿Borrar la base del modo libre? Se pierde todo lo que creaste.  y: sí · otra tecla: no",
        "Apagar o banco do modo livre? Tudo o que você criou será perdido.  y: sim · outra tecla: não",
        "删除自由模式的数据库？你创建的所有内容都会丢失。  y：是 · 其他键：否",
    ),
    (
        "Enter: run · Ctrl+R: erase the database · Esc: menu · PgUp/PgDn: scroll · Ctrl+Q: quit",
        "Enter: ejecutar · Ctrl+R: borrar la base · Esc: menú · PgUp/PgDn: desplazar · Ctrl+Q: salir",
        "Enter: executar · Ctrl+R: apagar o banco · Esc: menu · PgUp/PgDn: rolar · Ctrl+Q: sair",
        "Enter：执行 · Ctrl+R：删除数据库 · Esc：菜单 · PgUp/PgDn：滚动 · Ctrl+Q：退出",
    ),
];

pub fn lookup(lang: Lang, en: &str) -> Option<&'static str> {
    STRINGS
        .iter()
        .find(|(e, ..)| *e == en)
        .map(|(e, es, pt, zh)| match lang {
            Lang::En => *e,
            Lang::Es => *es,
            Lang::Pt => *pt,
            Lang::Zh => *zh,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cada `tr(lang, "en")` del código tiene que estar en la tabla.
    #[test]
    fn every_tr_call_in_the_sources_is_translated() {
        let mut missing = Vec::new();
        let mut calls = 0;
        for source in [include_str!("app.rs"), include_str!("ui.rs")] {
            let mut rest = source;
            while let Some(pos) = rest.find("tr(") {
                // `tr(` precedido por una letra es otra función (por ejemplo `str(`).
                let before = rest[..pos].chars().last();
                rest = &rest[pos + 3..];
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let args = rest.trim_start();
                let Some(after_lang) = args.find(',').map(|i| &args[i + 1..]) else {
                    continue;
                };
                let Some(literal) = first_string_literal(after_lang.trim_start()) else {
                    continue;
                };
                calls += 1;
                if Lang::ALL.iter().any(|l| lookup(*l, &literal).is_none()) {
                    missing.push(literal);
                }
            }
        }
        assert!(calls > 30, "the scan found only {calls} tr() calls");
        assert!(
            missing.is_empty(),
            "UI strings missing in some language: {missing:#?}"
        );
    }

    /// Lee un literal `"..."` al principio de `text` (con escapes simples).
    fn first_string_literal(text: &str) -> Option<String> {
        let mut chars = text.chars();
        if chars.next()? != '"' {
            return None;
        }
        let mut out = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => out.push(match chars.next()? {
                    'n' => '\n',
                    other => other,
                }),
                c => out.push(c),
            }
        }
        None
    }

    #[test]
    fn all_entries_have_text_in_both_languages() {
        for (en, es, pt, zh) in STRINGS {
            assert!(
                !en.is_empty() && !es.is_empty() && !pt.is_empty() && !zh.is_empty(),
                "{en}"
            );
        }
    }
}
