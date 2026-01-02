//! Bytecode interpreter for the toy language.

mod bytecode_reader;
mod error;
mod frame;
mod value;
mod vm;

pub use error::RuntimeError;
pub use value::{GcStats, Heap, Value};

use compile::CompiledModule;
use vm::Vm;

/// Execute a compiled module and return the result with the heap.
pub fn run(module: &CompiledModule) -> Result<(Value, Heap), RuntimeError> {
    let vm = Vm::new(module);
    vm.execute()
}

#[cfg(test)]
mod tests {
    use super::*;
    use compile::compile;
    use mir::{Block, BlockId, FuncId, Function, Inst, Module, Operand, VReg};

    fn make_module(insts: Vec<Inst>, return_vreg: Option<VReg>) -> Module {
        let mut func = Function::new(FuncId(0), Some("main".to_string()));
        let mut block = Block::new(BlockId(0));
        for inst in insts {
            block.push(inst);
        }
        block.push(Inst::Return {
            value: return_vreg.map(Operand::VReg),
        });
        func.blocks.push(block);
        func.vreg_count = 10;
        func.local_count = 10;
        Module {
            functions: vec![func],
            main_id: FuncId(0),
            struct_metadata: vec![],
        }
    }

    #[test]
    fn test_add_int() {
        let mir = make_module(
            vec![Inst::AddInt {
                dst: VReg(0),
                lhs: Operand::IntConst(2),
                rhs: Operand::IntConst(3),
            }],
            Some(VReg(0)),
        );
        let compiled = compile(&mir);
        let (result, _) = run(&compiled).unwrap();
        assert_eq!(result.as_int(), Some(5));
    }

    #[test]
    fn test_local_vars() {
        use mir::LocalId;
        let mir = make_module(
            vec![
                Inst::StoreLocal {
                    local: LocalId(0),
                    src: Operand::IntConst(42),
                },
                Inst::LoadLocal {
                    dst: VReg(0),
                    local: LocalId(0),
                },
            ],
            Some(VReg(0)),
        );
        let compiled = compile(&mir);
        let (result, _) = run(&compiled).unwrap();
        assert_eq!(result.as_int(), Some(42));
    }

    #[test]
    fn test_division_by_zero() {
        let mir = make_module(
            vec![Inst::DivInt {
                dst: VReg(0),
                lhs: Operand::IntConst(10),
                rhs: Operand::IntConst(0),
            }],
            Some(VReg(0)),
        );
        let compiled = compile(&mir);
        assert!(matches!(run(&compiled), Err(RuntimeError::DivisionByZero)));
    }

    #[test]
    fn test_function_call() {
        // fn add(a, b) { a + b }
        // main: add(1, 2) -> 3
        use mir::LocalId;

        // Function: add(a, b) = a + b
        let mut add_func = Function::new(FuncId(0), Some("add".to_string()));
        add_func.param_count = 2;
        add_func.local_count = 2;
        let mut add_block = Block::new(BlockId(0));
        add_block.push(Inst::LoadLocal {
            dst: VReg(0),
            local: LocalId(0),
        });
        add_block.push(Inst::LoadLocal {
            dst: VReg(1),
            local: LocalId(1),
        });
        add_block.push(Inst::AddInt {
            dst: VReg(2),
            lhs: Operand::VReg(VReg(0)),
            rhs: Operand::VReg(VReg(1)),
        });
        add_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(2))),
        });
        add_func.blocks.push(add_block);
        add_func.vreg_count = 3;

        // Main function: call add(1, 2)
        let mut main_func = Function::new(FuncId(1), Some("main".to_string()));
        let mut main_block = Block::new(BlockId(0));
        main_block.push(Inst::Call {
            dst: Some(VReg(0)),
            func: FuncId(0),
            args: vec![Operand::IntConst(1), Operand::IntConst(2)],
        });
        main_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(0))),
        });
        main_func.blocks.push(main_block);
        main_func.vreg_count = 5;
        main_func.local_count = 0;

        let module = Module {
            functions: vec![add_func, main_func],
            main_id: FuncId(1),
            struct_metadata: vec![],
        };

        let compiled = compile(&module);
        let (result, _) = run(&compiled).unwrap();
        assert_eq!(result.as_int(), Some(3));
    }

    #[test]
    fn test_closure() {
        // fn make_adder(x) { fn(y) { x + y } }
        // main: make_adder(5)(10) -> 15
        use mir::LocalId;

        // Inner closure function: fn(y) { x + y }
        // x is capture[0], y is param[0]
        let mut inner_func = Function::new(FuncId(0), Some("inner".to_string()));
        inner_func.param_count = 1;
        inner_func.local_count = 1;
        inner_func.is_closure = true;
        let mut inner_block = Block::new(BlockId(0));
        inner_block.push(Inst::LoadCapture {
            dst: VReg(0),
            index: 0, // x
        });
        inner_block.push(Inst::LoadLocal {
            dst: VReg(1),
            local: LocalId(0), // y
        });
        inner_block.push(Inst::AddInt {
            dst: VReg(2),
            lhs: Operand::VReg(VReg(0)),
            rhs: Operand::VReg(VReg(1)),
        });
        inner_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(2))),
        });
        inner_func.blocks.push(inner_block);
        inner_func.vreg_count = 3;

        // make_adder function: fn(x) { closure(inner, [x]) }
        let mut make_adder = Function::new(FuncId(1), Some("make_adder".to_string()));
        make_adder.param_count = 1;
        make_adder.local_count = 1;
        let mut make_adder_block = Block::new(BlockId(0));
        make_adder_block.push(Inst::LoadLocal {
            dst: VReg(0),
            local: LocalId(0), // x
        });
        make_adder_block.push(Inst::MakeClosure {
            dst: VReg(1),
            func: FuncId(0),
            captures: vec![Operand::VReg(VReg(0))],
        });
        make_adder_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(1))),
        });
        make_adder.blocks.push(make_adder_block);
        make_adder.vreg_count = 5;

        // Main: make_adder(5)(10)
        let mut main_func = Function::new(FuncId(2), Some("main".to_string()));
        let mut main_block = Block::new(BlockId(0));
        main_block.push(Inst::Call {
            dst: Some(VReg(0)),
            func: FuncId(1),
            args: vec![Operand::IntConst(5)],
        });
        main_block.push(Inst::CallIndirect {
            dst: Some(VReg(1)),
            callee: Operand::VReg(VReg(0)),
            args: vec![Operand::IntConst(10)],
        });
        main_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(1))),
        });
        main_func.blocks.push(main_block);
        main_func.vreg_count = 5;
        main_func.local_count = 0;

        let module = Module {
            functions: vec![inner_func, make_adder, main_func],
            main_id: FuncId(2),
            struct_metadata: vec![],
        };

        let compiled = compile(&module);
        let (result, _) = run(&compiled).unwrap();
        assert_eq!(result.as_int(), Some(15));
    }
}
