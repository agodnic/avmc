use crate::diag;
use crate::typed_ast;

/// Reports every call that closes a cycle in `program`'s call graph.
///
/// Returns false if it reported any.
pub fn check(program: &typed_ast::Program, diags: &mut diag::Sink) -> bool {
    let graph = call_graph(program);
    let mut states = vec![State::Unvisited; graph.len()];
    let mut ok = true;

    for index in 0..graph.len() {
        ok &= visit(index, program, &graph, &mut states, diags);
    }

    ok
}

/// A call, as an edge of the call graph.
struct Edge {
    /// The callee's position in the program.
    callee: usize,
    /// The span of the call expression.
    span: diag::Span,
}

/// Where the depth-first search stands on one function.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Unvisited,
    /// On the path the search is following: a call to it closes a cycle.
    OnPath,
    Done,
}

/// The calls each function makes, indexed like `program.funcs`.
fn call_graph(program: &typed_ast::Program) -> Vec<Vec<Edge>> {
    program
        .funcs
        .iter()
        .map(|func| {
            let mut edges = Vec::new();
            for stmt in &func.body {
                let (typed_ast::Stmt::Var { init: expr, .. }
                | typed_ast::Stmt::Return { expr, .. }) = stmt;
                collect(expr, &mut edges);
            }
            edges
        })
        .collect()
}

/// Collects the calls `expr` makes, the outer call before the calls in its
/// arguments, and otherwise left to right.
fn collect(expr: &typed_ast::Expr, edges: &mut Vec<Edge>) {
    match &expr.kind {
        typed_ast::ExprKind::IntLit(_)
        | typed_ast::ExprKind::BoolLit(_)
        | typed_ast::ExprKind::Var(_)
        | typed_ast::ExprKind::Param(_) => {}
        typed_ast::ExprKind::Binary { lhs, rhs, .. } => {
            collect(lhs, edges);
            collect(rhs, edges);
        }
        typed_ast::ExprKind::Unary { operand, .. } => collect(operand, edges),
        typed_ast::ExprKind::Call { callee, args } => {
            if let Ok(callee) = usize::try_from(callee.0) {
                edges.push(Edge {
                    callee,
                    span: expr.span,
                });
            }
            for arg in args {
                collect(arg, edges);
            }
        }
    }
}

/// Follows every call the function at `index` makes, depth first, reporting
/// the ones that reach a function already on the path.
///
/// Returns false if it reported any.
fn visit(
    index: usize,
    program: &typed_ast::Program,
    graph: &[Vec<Edge>],
    states: &mut [State],
    diags: &mut diag::Sink,
) -> bool {
    // A function is searched once: a second search would meet the same calls.
    let (Some(edges), Some(State::Unvisited)) = (graph.get(index), states.get(index)) else {
        return true;
    };
    mark(index, State::OnPath, states);

    let mut ok = true;
    for edge in edges {
        if states.get(edge.callee) == Some(&State::OnPath) {
            report(edge, program, diags);
            ok = false;
        } else {
            ok &= visit(edge.callee, program, graph, states, diags);
        }
    }

    mark(index, State::Done, states);
    ok
}

/// Records where the search stands on the function at `index`.
fn mark(index: usize, state: State, states: &mut [State]) {
    if let Some(slot) = states.get_mut(index) {
        *slot = state;
    }
}

/// Reports a call that closes a cycle, naming the callee.
fn report(edge: &Edge, program: &typed_ast::Program, diags: &mut diag::Sink) {
    let Some(callee) = program.funcs.get(edge.callee) else {
        return;
    };
    diags.push(diag::Entry {
        kind: diag::Kind::RecursiveCall {
            name: callee.name.text.clone(),
        },
        span: edge.span,
    });
}
