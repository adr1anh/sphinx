use core::marker::PhantomData;
use std::collections::HashMap;

use crate::ir::{Config, DslIR};

const GNARK_TEMPLATE: &str = include_str!("lib/template.txt");

/// Indents a block of lines by one tab.
pub fn indent(lines: Vec<String>) -> Vec<String> {
    lines.into_iter().map(|x| format!("\t{}", x)).collect()
}

/// Masks the evaluation of lines based on a condition.
pub fn mask(cond: &str, lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let parts = line.split('=').collect::<Vec<_>>();
            let dst = parts[0].trim_end();
            let expr = parts[1].trim_end();
            if dst.contains("var") {
                format!("{dst} = api.Select({cond}, {expr}, {dst}")
            } else if dst.contains("felt") {
                format!("{dst} = fieldChip.Select({cond}, {expr}, {dst})")
            } else if dst.contains("ext") {
                format!("{dst} = fieldChip.SelectExtension({cond}, {expr}, {dst}")
            } else {
                panic!("unexpected dst for DslIR::IfEq")
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct GnarkBackend<C: Config> {
    pub nb_backend_vars: usize,
    pub used: HashMap<String, bool>,
    pub phantom: PhantomData<C>,
}

impl<C: Config> GnarkBackend<C> {
    pub fn alloc(&mut self) -> String {
        let id = format!("backend{}", self.nb_backend_vars);
        self.nb_backend_vars += 1;
        id
    }

    pub fn assign(&mut self, id: String) -> &str {
        self.used.insert(id, true);
        "="
    }

    pub fn emit(&mut self, operations: Vec<DslIR<C>>) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for instruction in operations {
            match instruction {
                DslIR::Imm(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!("{} {} frontend.Variable({})", a.id(), operator, b));
                }
                DslIR::ImmFelt(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} babybear.NewVariable({})",
                        a.id(),
                        operator,
                        b
                    ));
                }
                DslIR::ImmExt(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} babybear.NewExtensionVariable({})",
                        a.id(),
                        operator,
                        b
                    ));
                }
                DslIR::AddV(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Add({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ))
                }
                DslIR::AddVI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Add({}, frontend.Variable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::AddF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Add({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::AddFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Add({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::AddE(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.AddExtension({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::AddEI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.AddExtension({}, babybear.NewExtensionVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::AddEFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.AddFelt({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::AddEFFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.AddFelt(babybear.NewExtensionVariable({}), {})",
                        a.id(),
                        operator,
                        c,
                        b.id()
                    ));
                }
                DslIR::AddEF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.AddFelt({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::MulV(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Mul({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::MulVI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Mul(frontend.Variable({}), {})",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::MulF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Mul({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::MulFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Mul({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::MulE(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.MulExtension({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::MulEI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.MulExtension({}, babybear.NewExtensionVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::MulEFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.MulFelt({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        c,
                        b.id()
                    ));
                }
                DslIR::MulEF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Mul({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::SubV(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Sub({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::SubVI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Sub(frontend.Variable({}), {})",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::SubVIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} api.Sub(frontend.Variable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::SubF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Sub({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::SubFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Sub({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::SubFIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Sub(babybear.NewVariable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::SubE(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.SubExtension({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::SubEI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.SubExtension({}, babybear.NewExtensionVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::SubEIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.SubExtension(babybear.NewExtensionVariable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::SubEFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.SubFelt({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        c,
                        b.id()
                    ));
                }
                DslIR::SubEFIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.SubFelt(babybear.NewExtensionVariable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::SubEF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.SubFelt({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::DivF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Div({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::DivFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Div({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::DivFIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.Div(babybear.NewVariable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::DivE(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.DivExtension({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::DivEI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.DivExtension({}, babybear.NewExtensionVariable({}))",
                        a.id(),
                        operator,
                        b.id(),
                        c
                    ));
                }
                DslIR::DivEIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.DivExtension(babybear.NewExtensionVariable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::DivEFI(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.DivFelt({}, babybear.NewVariable({}))",
                        a.id(),
                        operator,
                        c,
                        b.id()
                    ));
                }
                DslIR::DivEFIN(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.DivFelt(babybear.NewExtensionVariable({}), {})",
                        a.id(),
                        operator,
                        b,
                        c.id()
                    ));
                }
                DslIR::DivEF(a, b, c) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.DivFelt({}, {})",
                        a.id(),
                        operator,
                        b.id(),
                        c.id()
                    ));
                }
                DslIR::NegV(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!("{} {} api.Neg({})", a.id(), operator, b.id()));
                }
                DslIR::NegF(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!("{} {} fieldChip.Neg({})", a.id(), operator, b.id()));
                }
                DslIR::NegE(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.NegExtension({})",
                        a.id(),
                        operator,
                        b.id()
                    ));
                }
                DslIR::InvV(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!("{} {} api.Inv({})", a.id(), operator, b.id()));
                }
                DslIR::InvF(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!("{} {} fieldChip.Inv({})", a.id(), operator, b.id()));
                }
                DslIR::InvE(a, b) => {
                    let operator = self.assign(a.id());
                    lines.push(format!(
                        "{} {} fieldChip.InvExtension({})",
                        a.id(),
                        operator,
                        b.id()
                    ))
                }
                DslIR::For(a, b, _, d) => {
                    lines.push(format!("for i := {}; i < {}; i++ {{", a.value(), b.value()));
                    lines.extend(indent(self.emit(d)));
                    lines.push("}".to_string());
                }
                DslIR::IfEq(a, b, c, d) => {
                    let cond = self.alloc();
                    let operator = self.assign(cond.clone());
                    lines.push(format!(
                        "{} {} api.IsZero(api.Sub({}, {}))",
                        cond,
                        operator,
                        a.id(),
                        b.id()
                    ));
                    lines.extend(mask(&cond, &self.emit(c)));
                    lines.extend(mask(&cond, &self.emit(d)));
                }
                DslIR::IfNe(a, b, c, d) => {
                    let cond = self.alloc();
                    let operator = self.assign(cond.clone());
                    lines.push(format!(
                        "{} {} api.Sub(frontend.Variable(1), api.IsZero(api.Sub({}, {})))",
                        cond,
                        operator,
                        a.id(),
                        b.id()
                    ));
                    lines.extend(mask(&cond, &self.emit(c)));
                    lines.extend(mask(&cond, &self.emit(d)));
                }
                DslIR::IfEqI(a, b, c, d) => {
                    let cond = self.alloc();
                    let operator = self.assign(cond.clone());
                    lines.push(format!(
                        "{} {} api.IsZero(api.Sub({}, frontend.Variable({})))",
                        cond,
                        operator,
                        a.id(),
                        b
                    ));
                    lines.extend(mask(&cond, &self.emit(c)));
                    lines.extend(mask(&cond, &self.emit(d)));
                }
                DslIR::IfNeI(a, b, c, d) => {
                    let cond = self.alloc();
                    let operator = self.assign(cond.clone());
                    lines.push(format!(
                        "{} {} api.Sub(frontend.Variable(1), api.IsZero(api.Sub({}, frontend.Variable({}))))",
                        cond,
                        operator,
                        a.id(),
                        b
                    ));
                    lines.extend(mask(&cond, &self.emit(c)));
                    lines.extend(mask(&cond, &self.emit(d)));
                }
                DslIR::AssertEqV(a, b) => {
                    lines.push(format!("api.AssertEq({}, {})", a.id(), b.id()));
                }
                DslIR::AssertNeV(a, b) => {
                    lines.push(format!("api.AssertNe({}, {})", a.id(), b.id()));
                }
                DslIR::AssertEqF(a, b) => {
                    lines.push(format!("fieldChip.AssertEq({}, {})", a.id(), b.id()));
                }
                DslIR::AssertNeF(a, b) => {
                    lines.push(format!("fieldChip.AssertNe({}, {})", a.id(), b.id()));
                }
                DslIR::AssertEqE(a, b) => {
                    lines.push(format!(
                        "fieldChip.AssertEqExtension({}, {})",
                        a.id(),
                        b.id()
                    ));
                }
                DslIR::AssertNeE(a, b) => {
                    lines.push(format!(
                        "fieldChip.AssertNeExtension({}, {})",
                        a.id(),
                        b.id()
                    ));
                }
                DslIR::AssertEqVI(a, b) => {
                    lines.push(format!(
                        "api.AssertEq({}, frontend.Variable({}))",
                        a.id(),
                        b
                    ));
                }
                DslIR::AssertNeVI(a, b) => {
                    lines.push(format!(
                        "api.AssertNe({}, frontend.Variable({}))",
                        a.id(),
                        b
                    ));
                }
                DslIR::AssertEqFI(a, b) => {
                    lines.push(format!(
                        "fieldChip.AssertEq({}, babybear.NewVariable({}))",
                        a.id(),
                        b
                    ));
                }
                DslIR::AssertNeFI(a, b) => {
                    lines.push(format!(
                        "fieldChip.AssertNe({}, babybear.NewVariable({}))",
                        a.id(),
                        b
                    ));
                }
                DslIR::AssertEqEI(a, b) => {
                    lines.push(format!(
                        "fieldChip.AssertEqExtension({}, babybear.NewExtensionVariable({}))",
                        a.id(),
                        b
                    ));
                }
                DslIR::AssertNeEI(a, b) => {
                    lines.push(format!(
                        "fieldChip.AssertNeExtension({}, babybear.NewExtensionVariable({}))",
                        a.id(),
                        b
                    ));
                }
                _ => todo!(),
            };
        }
        lines
    }

    pub fn compile(&mut self, program: Vec<DslIR<C>>) -> String {
        let operations = self.emit(program);
        let initializes = self
            .used
            .keys()
            .map(|id| {
                if id.contains("var") {
                    format!("var {} frontend.Variable", id)
                } else if id.contains("felt") {
                    format!("var {} *babybear.Variable", id)
                } else if id.contains("ext") {
                    format!("var {} *babybear.ExtensionVariable", id)
                } else if id.contains("backend") {
                    format!("var {} frontend.Variable", id)
                } else {
                    panic!("Unknown variable type")
                }
            })
            .collect::<Vec<_>>();

        let mut lines = Vec::new();
        lines.extend(vec!["".to_string(), "// Variables.".to_string()]);
        lines.extend(initializes);
        lines.extend(vec!["".to_string(), "// Operations.".to_string()]);
        lines.extend(operations);
        GNARK_TEMPLATE.replace("{{LINES}}", &indent(lines).join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use p3_baby_bear::BabyBear;
    use p3_field::{extension::BinomialExtensionField, AbstractField};

    use crate::ir::{Felt, Usize, Var};
    use crate::prelude::Builder;

    use super::*;

    #[derive(Clone)]
    struct BabyBearConfig;

    impl Config for BabyBearConfig {
        type N = BabyBear;
        type F = BabyBear;
        type EF = BinomialExtensionField<BabyBear, 4>;
    }

    #[test]
    fn test() {
        let program = vec![
            DslIR::Imm(Var::new(0), BabyBear::zero()),
            DslIR::AddV(Var::new(3), Var::new(0), Var::new(1)),
            DslIR::AddVI(Var::new(4), Var::new(3), BabyBear::one()),
            DslIR::Imm(Var::new(1), BabyBear::one()),
            DslIR::AddV(Var::new(2), Var::new(0), Var::new(1)),
            DslIR::AddVI(Var::new(3), Var::new(2), BabyBear::one()),
            DslIR::SubV(Var::new(4), Var::new(2), Var::new(3)),
            DslIR::SubVI(Var::new(5), Var::new(4), BabyBear::one()),
            DslIR::MulV(Var::new(6), Var::new(2), Var::new(5)),
            DslIR::MulVI(Var::new(7), Var::new(6), BabyBear::one()),
            DslIR::NegV(Var::new(9), Var::new(8)),
            DslIR::InvV(Var::new(10), Var::new(9)),
        ];
        let mut backend = GnarkBackend::<BabyBearConfig> {
            nb_backend_vars: 0,
            used: HashMap::new(),
            phantom: PhantomData,
        };
        let result = backend.compile(program);
        println!("{:?}", result);
    }

    #[test]
    fn test2() {
        let mut builder = Builder::<BabyBearConfig>::default();
        let t: Var<_> = builder.eval(BabyBear::zero());
        let a: Felt<_> = builder.eval(BabyBear::zero());
        let b: Felt<_> = builder.eval(BabyBear::one());

        let start = Usize::Const(0);
        let end = Usize::Const(12);

        builder.range(start, end).for_each(|_, builder| {
            let temp: Felt<_> = builder.uninit();
            builder.assign(&temp, b);
            builder.assign(&b, a + b);
            builder.assign(&a, temp);
        });

        let expected_value = BabyBear::from_canonical_u32(144);
        builder.assert_felt_eq(a, expected_value);

        builder.if_eq(t, t).then_or_else(
            |builder| {
                builder.assign(&a, b);
            },
            |builder| {
                builder.assign(&a, a + b);
            },
        );
        builder.if_ne(t, t).then_or_else(
            |builder| {
                builder.assign(&a, b);
            },
            |builder| {
                builder.assign(&a, a + b);
            },
        );

        let mut backend = GnarkBackend::<BabyBearConfig> {
            nb_backend_vars: 0,
            used: HashMap::new(),
            phantom: PhantomData,
        };
        let result = backend.compile(builder.operations);

        // Write to file.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}/src/gnark/lib/main.go", manifest_dir);
        let mut file = File::create(path).unwrap();
        file.write_all(result.as_bytes()).unwrap();
    }
}
