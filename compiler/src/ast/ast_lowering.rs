use crate::ast::{BinOp, Decl, Expr, MatchPattern, Pattern, Script, Stmt, UnOp};
use crate::types::Type;
use pulse_ast::Constant;

pub fn lower_script_to_source(script: &Script) -> String {
    let mut out = String::new();
    for decl in &script.declarations {
        emit_decl(decl, 0, &mut out);
    }
    out
}

fn emit_decl(decl: &Decl, indent: usize, out: &mut String) {
    match decl {
        Decl::Function(name, params, ret, body) => {
            emit_indent(indent, out);
            out.push_str("fn ");
            out.push_str(name);
            out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                if let Some(ty) = &p.type_annotation {
                    out.push_str(": ");
                    out.push_str(&emit_type(ty));
                }
            }
            out.push(')');
            if let Some(ty) = ret {
                out.push_str(" -> ");
                out.push_str(&emit_type(ty));
            }
            out.push_str(" {\n");
            for stmt in body {
                emit_stmt(stmt, indent + 1, out);
            }
            emit_indent(indent, out);
            out.push_str("}\n");
        }
        Decl::Class(name, parent, members) => {
            emit_indent(indent, out);
            out.push_str("class ");
            out.push_str(name);
            if let Some(p) = parent {
                out.push_str(" extends ");
                out.push_str(p);
            }
            out.push_str(" {\n");
            for m in members {
                match m {
                    Decl::Function(name, params, ret, body) => {
                        emit_indent(indent + 1, out);
                        out.push_str("fn ");
                        out.push_str(name);
                        out.push('(');
                        // ParserV2 adds implicit `this` parameter for methods.
                        // Compiler parser expects no explicit `this` in method signatures.
                        let start = if params
                            .first()
                            .is_some_and(|p| p.name.eq_ignore_ascii_case("this"))
                        {
                            1
                        } else {
                            0
                        };
                        for (i, p) in params.iter().skip(start).enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(&p.name);
                            if let Some(ty) = &p.type_annotation {
                                out.push_str(": ");
                                out.push_str(&emit_type(ty));
                            }
                        }
                        out.push(')');
                        if let Some(ty) = ret {
                            out.push_str(" -> ");
                            out.push_str(&emit_type(ty));
                        }
                        out.push_str(" {\n");
                        for stmt in body {
                            emit_stmt(stmt, indent + 2, out);
                        }
                        emit_indent(indent + 1, out);
                        out.push_str("}\n");
                    }
                    _ => emit_decl(m, indent + 1, out),
                }
            }
            emit_indent(indent, out);
            out.push_str("}\n");
        }
        Decl::Actor(name, body) => {
            emit_indent(indent, out);
            out.push_str("actor ");
            out.push_str(name);
            out.push_str(" {\n");
            for stmt in body {
                emit_stmt(stmt, indent + 1, out);
            }
            emit_indent(indent, out);
            out.push_str("}\n");
        }
        Decl::SharedMemory(name, init) => {
            emit_indent(indent, out);
            out.push_str("shared memory ");
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&emit_expr(init));
            out.push_str(";\n");
        }
        Decl::Stmt(stmt) => emit_stmt(stmt, indent, out),
    }
}

fn emit_stmt(stmt: &Stmt, indent: usize, out: &mut String) {
    match stmt {
        Stmt::Expression(expr) => {
            emit_indent(indent, out);
            out.push_str(&emit_expr(expr));
            out.push_str(";\n");
        }
        Stmt::Print(expr) => {
            emit_indent(indent, out);
            out.push_str("print(");
            out.push_str(&emit_expr(expr));
            out.push_str(");\n");
        }
        Stmt::Let(name, ann, init) => {
            emit_indent(indent, out);
            out.push_str("let ");
            out.push_str(name);
            if let Some(ty) = ann {
                out.push_str(": ");
                out.push_str(&emit_type(ty));
            }
            if let Some(expr) = init {
                out.push_str(" = ");
                out.push_str(&emit_expr(expr));
            }
            out.push_str(";\n");
        }
        Stmt::Const(name, ann, expr) => {
            emit_indent(indent, out);
            out.push_str("const ");
            out.push_str(name);
            if let Some(ty) = ann {
                out.push_str(": ");
                out.push_str(&emit_type(ty));
            }
            out.push_str(" = ");
            out.push_str(&emit_expr(expr));
            out.push_str(";\n");
        }
        Stmt::If(cond, then_branch, else_branch, _) => {
            emit_indent(indent, out);
            out.push_str("if (");
            out.push_str(&emit_expr(cond));
            out.push_str(") ");
            emit_stmt_blockish(then_branch, indent, out);
            if let Some(else_b) = else_branch {
                emit_indent(indent, out);
                out.push_str("else ");
                emit_stmt_blockish(else_b, indent, out);
            }
        }
        Stmt::While(cond, body) => {
            emit_indent(indent, out);
            out.push_str("while (");
            out.push_str(&emit_expr(cond));
            out.push_str(") ");
            emit_stmt_blockish(body, indent, out);
        }
        Stmt::For(init, cond, update, body) => {
            emit_indent(indent, out);
            out.push_str("for (");
            if let Some(init_stmt) = init {
                out.push_str(&emit_for_init(init_stmt));
            } else {
                out.push(';');
            }
            out.push(' ');
            if let Some(c) = cond {
                out.push_str(&emit_expr(c));
            }
            out.push_str("; ");
            if let Some(u) = update {
                out.push_str(&emit_expr(u));
            }
            out.push_str(") ");
            emit_stmt_blockish(body, indent, out);
        }
        Stmt::Return(expr) => {
            emit_indent(indent, out);
            out.push_str("return");
            if let Some(e) = expr {
                out.push(' ');
                out.push_str(&emit_expr(e));
            }
            out.push_str(";\n");
        }
        Stmt::Break => {
            emit_indent(indent, out);
            out.push_str("break;\n");
        }
        Stmt::Continue => {
            emit_indent(indent, out);
            out.push_str("continue;\n");
        }
        Stmt::Block(stmts) => {
            // Recover ParserV2 desugared `for .. in` form back into native syntax
            // so bytecode generation keeps original loop semantics.
            if let Some((var_name, iterable, body)) = try_recover_for_in(stmts) {
                emit_indent(indent, out);
                out.push_str("for ");
                out.push_str(var_name);
                out.push_str(" in ");
                out.push_str(&emit_expr(iterable));
                out.push(' ');
                emit_stmt_blockish(body, indent, out);
                return;
            }
            emit_indent(indent, out);
            out.push_str("{\n");
            for s in stmts {
                emit_stmt(s, indent + 1, out);
            }
            emit_indent(indent, out);
            out.push_str("}\n");
        }
        Stmt::Try(body, catch_var, catch_body) => {
            emit_indent(indent, out);
            out.push_str("try ");
            emit_stmt_blockish(body, indent, out);
            emit_indent(indent, out);
            out.push_str("catch ");
            out.push_str(catch_var);
            out.push(' ');
            emit_stmt_blockish(catch_body, indent, out);
        }
        Stmt::Throw(expr) => {
            emit_indent(indent, out);
            out.push_str("throw ");
            out.push_str(&emit_expr(expr));
            out.push_str(";\n");
        }
        Stmt::Send(target, msg) => {
            emit_indent(indent, out);
            out.push_str("send(");
            out.push_str(&emit_expr(target));
            out.push_str(", ");
            out.push_str(&emit_expr(msg));
            out.push_str(");\n");
        }
        Stmt::Link(expr) => {
            emit_indent(indent, out);
            out.push_str("link(");
            out.push_str(&emit_expr(expr));
            out.push_str(");\n");
        }
        Stmt::Monitor(expr) => {
            emit_indent(indent, out);
            out.push_str("monitor(");
            out.push_str(&emit_expr(expr));
            out.push_str(");\n");
        }
        Stmt::Spawn(expr) => {
            emit_indent(indent, out);
            out.push_str("spawn ");
            out.push_str(&emit_expr(expr));
            out.push_str(";\n");
        }
        Stmt::Import(path, alias) => {
            emit_indent(indent, out);
            out.push_str("import ");
            out.push_str(&emit_string(path));
            if let Some(a) = alias {
                out.push_str(" as ");
                out.push_str(a);
            }
            out.push_str(";\n");
        }
        Stmt::Receive(arms) => {
            emit_indent(indent, out);
            out.push_str("receive {\n");
            for (i, (p, e)) in arms.iter().enumerate() {
                emit_indent(indent + 1, out);
                out.push_str(&emit_pattern(p));
                out.push_str(" => ");
                out.push_str(&emit_expr(e));
                if i + 1 < arms.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            emit_indent(indent, out);
            out.push_str("};\n");
        }
        Stmt::Match(expr, arms) => {
            emit_indent(indent, out);
            out.push_str("match ");
            out.push_str(&emit_expr(expr));
            out.push_str(" {\n");
            for (i, (p, s)) in arms.iter().enumerate() {
                emit_indent(indent + 1, out);
                out.push_str(&emit_match_pattern(p));
                out.push_str(" => ");
                match s {
                    Stmt::Expression(e) => out.push_str(&emit_expr(e)),
                    _ => {
                        out.push_str("{\n");
                        emit_stmt(s, indent + 2, out);
                        emit_indent(indent + 1, out);
                        out.push('}');
                    }
                }
                if i + 1 < arms.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            emit_indent(indent, out);
            out.push_str("};\n");
        }
    }
}

fn emit_stmt_blockish(stmt: &Stmt, indent: usize, out: &mut String) {
    match stmt {
        Stmt::Block(_) => emit_stmt(stmt, indent, out),
        _ => {
            out.push_str("{\n");
            emit_stmt(stmt, indent + 1, out);
            emit_indent(indent, out);
            out.push_str("}\n");
        }
    }
}

fn emit_for_init(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let(name, ann, init) => {
            let mut s = format!("let {}", name);
            if let Some(ty) = ann {
                s.push_str(": ");
                s.push_str(&emit_type(ty));
            }
            if let Some(expr) = init {
                s.push_str(" = ");
                s.push_str(&emit_expr(expr));
            }
            s.push(';');
            s
        }
        Stmt::Const(name, ann, init) => {
            let mut s = format!("const {}", name);
            if let Some(ty) = ann {
                s.push_str(": ");
                s.push_str(&emit_type(ty));
            }
            s.push_str(" = ");
            s.push_str(&emit_expr(init));
            s.push(';');
            s
        }
        Stmt::Expression(expr) => format!("{};", emit_expr(expr)),
        _ => ";".to_string(),
    }
}

fn emit_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(c) => emit_constant(c),
        Expr::Variable(n) => n.clone(),
        Expr::Binary(l, op, r) => {
            format!("({} {} {})", emit_expr(l), emit_bin_op(*op), emit_expr(r))
        }
        Expr::Unary(op, e) => format!("({}{})", emit_un_op(*op), emit_expr(e)),
        Expr::Call(callee, args) => {
            let mut s = format!("{}(", emit_expr(callee));
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_expr(a));
            }
            s.push(')');
            s
        }
        Expr::Get(obj, name) => format!("{}.{}", emit_expr(obj), name),
        Expr::Set(obj, name, value) => {
            format!("{}.{} = {}", emit_expr(obj), name, emit_expr(value))
        }
        Expr::Index(obj, index) => format!("{}[{}]", emit_expr(obj), emit_expr(index)),
        Expr::IndexSet(obj, index, value) => {
            format!(
                "{}[{}] = {}",
                emit_expr(obj),
                emit_expr(index),
                emit_expr(value)
            )
        }
        Expr::This => "this".to_string(),
        Expr::Super(name) => format!("super.{}", name),
        Expr::List(items) => {
            let mut s = String::from("[");
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_expr(it));
            }
            s.push(']');
            s
        }
        Expr::Map(entries) => {
            let mut s = String::from("{");
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_expr(k));
                s.push_str(": ");
                s.push_str(&emit_expr(v));
            }
            s.push('}');
            s
        }
        Expr::Closure(_name, params, ret, body) => {
            let mut s = String::from("fn(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&p.name);
                if let Some(ty) = &p.type_annotation {
                    s.push_str(": ");
                    s.push_str(&emit_type(ty));
                }
            }
            s.push(')');
            if let Some(ty) = ret {
                s.push_str(" -> ");
                s.push_str(&emit_type(ty));
            }
            s.push_str(" { ");
            for st in body {
                // Compact lambda body; still valid.
                s.push_str(&emit_stmt_inline(st));
                s.push(' ');
            }
            s.push('}');
            s
        }
        Expr::Assign(name, expr) => format!("{} = {}", name, emit_expr(expr)),
        Expr::MethodCall(obj, name, args) => {
            let mut s = format!("{}.{}(", emit_expr(obj), name);
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_expr(a));
            }
            s.push(')');
            s
        }
        Expr::Receive(arms) => {
            let mut s = String::from("receive { ");
            for (i, (p, e)) in arms.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_pattern(p));
                s.push_str(" => ");
                s.push_str(&emit_expr(e));
            }
            s.push_str(" }");
            s
        }
        Expr::Spawn(expr) => format!("spawn {}", emit_expr(expr)),
        Expr::Send(a, b) => format!("({} ! {})", emit_expr(a), emit_expr(b)),
        Expr::ClassLiteral(_, _, _) => "nil".to_string(),
        Expr::TypeGuard(expr, ty) => format!("({} is {})", emit_expr(expr), emit_type(ty)),
        Expr::TypeCast(expr, ty) => format!("({} as {})", emit_expr(expr), emit_type(ty)),
        Expr::CompoundAssign(name, op, expr) => {
            format!("{} {}= {}", name, emit_bin_op(*op), emit_expr(expr))
        }
        Expr::Range(a, b) => format!("{}..{}", emit_expr(a), emit_expr(b)),
    }
}

fn emit_stmt_inline(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Expression(e) => format!("{};", emit_expr(e)),
        Stmt::Print(e) => format!("print({});", emit_expr(e)),
        Stmt::Let(name, ann, init) => {
            let mut s = format!("let {}", name);
            if let Some(ty) = ann {
                s.push_str(": ");
                s.push_str(&emit_type(ty));
            }
            if let Some(e) = init {
                s.push_str(" = ");
                s.push_str(&emit_expr(e));
            }
            s.push(';');
            s
        }
        Stmt::Const(name, ann, init) => {
            let mut s = format!("const {}", name);
            if let Some(ty) = ann {
                s.push_str(": ");
                s.push_str(&emit_type(ty));
            }
            s.push_str(" = ");
            s.push_str(&emit_expr(init));
            s.push(';');
            s
        }
        Stmt::Return(e) => {
            if let Some(v) = e {
                format!("return {};", emit_expr(v))
            } else {
                "return;".to_string()
            }
        }
        Stmt::Break => "break;".to_string(),
        Stmt::Continue => "continue;".to_string(),
        Stmt::Block(stmts) => {
            let mut s = String::from("{ ");
            for st in stmts {
                s.push_str(&emit_stmt_inline(st));
                s.push(' ');
            }
            s.push('}');
            s
        }
        Stmt::If(..)
        | Stmt::While(..)
        | Stmt::For(..)
        | Stmt::Try(..)
        | Stmt::Throw(..)
        | Stmt::Send(..)
        | Stmt::Link(..)
        | Stmt::Monitor(..)
        | Stmt::Spawn(..)
        | Stmt::Import(..)
        | Stmt::Receive(..)
        | Stmt::Match(..) => "nil;".to_string(),
    }
}

fn try_recover_for_in(stmts: &[Stmt]) -> Option<(&str, &Expr, &Stmt)> {
    if stmts.len() != 2 {
        return None;
    }
    let iterable = match &stmts[0] {
        Stmt::Expression(expr) => expr,
        _ => return None,
    };
    match &stmts[1] {
        Stmt::For(Some(init), None, None, body) => {
            if let Stmt::Let(name, None, None) = init.as_ref() {
                Some((name.as_str(), iterable, body.as_ref()))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn emit_pattern(p: &Pattern) -> String {
    match p {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Variable(v) => v.clone(),
        Pattern::Literal(e) => emit_expr(e),
    }
}

fn emit_match_pattern(p: &MatchPattern) -> String {
    match p {
        MatchPattern::Wildcard => "_".to_string(),
        MatchPattern::Variable(v) => v.clone(),
        MatchPattern::Literal(c) => emit_constant(c),
        MatchPattern::Range(a, b) => format!("{}..{}", emit_constant(a), emit_constant(b)),
        MatchPattern::TypePattern(name, ty) => format!("{}: {}", name, emit_type(ty)),
        MatchPattern::Constructor(name, args) => {
            let mut s = format!("{}(", name);
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_match_pattern(a));
            }
            s.push(')');
            s
        }
        MatchPattern::Or(a, b) => format!("{} | {}", emit_match_pattern(a), emit_match_pattern(b)),
    }
}

fn emit_constant(c: &Constant) -> String {
    match c {
        Constant::Int(i) => i.to_string(),
        Constant::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}.0", f)
            } else {
                f.to_string()
            }
        }
        Constant::Bool(b) => b.to_string(),
        Constant::String(s) => emit_string(s),
        Constant::Unit => "nil".to_string(),
        _ => "nil".to_string(),
    }
}

fn emit_string(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn emit_bin_op(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Union => "|",
    }
}

fn emit_un_op(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
    }
}

fn emit_type(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Unit => "Unit".to_string(),
        Type::Pid => "Pid".to_string(),
        Type::Any => "Any".to_string(),
        Type::Atomic => "Atomic".to_string(),
        Type::Var(v) => format!("T{}", v.id),
        Type::List(t) => format!("List<{}>", emit_type(t)),
        Type::Map(k, v) => format!("Map<{}, {}>", emit_type(k), emit_type(v)),
        Type::Fn(params, ret) => {
            let mut s = String::from("Fn<(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&emit_type(p));
            }
            s.push_str(") -> ");
            s.push_str(&emit_type(ret));
            s.push('>');
            s
        }
        Type::Custom(name) => name.clone(),
        Type::Union(types) => {
            let mut s = String::new();
            for (i, t) in types.iter().enumerate() {
                if i > 0 {
                    s.push_str(" | ");
                }
                s.push_str(&emit_type(t));
            }
            s
        }
        Type::Option(inner) => format!("Option<{}>", emit_type(inner)),
        Type::Effect(_) => "Any".to_string(),
        Type::Generic(name) => name.clone(),
    }
}

fn emit_indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("    ");
    }
}
