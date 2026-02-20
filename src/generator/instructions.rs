#![allow(unused)]
use super::*;
use crate::analysis::AnalyzeSummary;
use crate::nodes::Expr;
use crate::parser::Spanned;
use std::fmt;

macro_rules! instructions {
    (
    $([$name:ident] [$($ins:tt)*] [$($args:tt)*] [$($raw:tt)*] [$($doc:tt)*]);*
    $(;)?
    ) => {
        instructions!{@enum
            $([$name] [$($args)*] [$($doc)*]);*
        }

        instructions!{@convertion
            $([$name] [$($ins)*] [$($args)*]);*
        }

        instructions!{@impl
            $([$name] [$($ins)*] [$($args)*]);*
        }

        instructions!{@display
            $([$name] [$($ins)*] [$($args)*]);*
        }
    };

    ( @enum
    $([$name:ident] [$($arg:ident),*] [$doc:literal]);*
    ) => {
        #[derive(Debug, Clone)]
        pub enum Instruction {
            $(
                #[doc = $doc]
                $name( $(instructions!(@arg-ty [$arg])),* )
            ),*
        }
    };

    ( @impl
    $([$name:ident] [$ins:ident $($yunk:tt)?] [$($arg:ident),*]);*
    ) => {
        impl Instruction {
            pub fn is_instruction(name: &str) -> bool {
                match name {
                    $(stringify!($ins) => true,)*
                    _ => false
                }
            }

            pub fn scope(self, rhs: &str) -> Self {
                match self {
                    $(instructions!(@args-indexed [$name] [$($arg),*]) =>
                        instructions!(@args-offset [$name] [rhs] [$($arg),*]),
                    )*
                    _ => self
                }
            }
        }
    };

    ( @display
    $([$name:ident] [$ins:ident $($alias:tt)?] [$($arg:ident),*]);*
    ) => {
        impl fmt::Display for Instruction {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(instructions!(@args-indexed [$name] [$($arg),*]) => {
                        let inst = instructions!(@aliasing [$ins] [$($alias)?]);
                        instructions!(@args-format [f] [inst] [$($arg),*])
                    })*
                    _ => Ok(())
                }
            }
        }
    };

    ( @convertion
    $([$name:ident] [$ins:ident $($yunk:tt)?] [$($arg:ident),*]);*
    ) => {
        impl Instruction {
            pub fn from_0(summary: &mut AnalyzeSummary, name: Spanned<&str>) -> Option<Self> {
                match name.value {
                    $(stringify!($ins) if 0 == instructions!(@count [$($arg),*]) =>
                        Some(Self::$name($(instructions!(@arg-mock [$arg])),*)),
                      stringify!($ins) => None,
                    )*
                    _ => {
                        summary.error(AnalyzeInsNotExist { location: name.span });
                        None
                    }
                }
            }

            pub fn from_1(summary: &mut AnalyzeSummary, name: Spanned<&str>, arg0: &Expr) -> Option<Self> {
                match name.value {
                    $(stringify!($ins) if 1 == instructions!(@count [$($arg),*]) =>
                        Some(instructions!(@args [summary] [Self::$name] [1] [arg0] [$($arg),*])),
                    )*
                    _ => {
                        summary.error(AnalyzeInsNotExist { location: name.span });
                        None
                    }
                }
            }

            pub fn from_2(summary: &mut AnalyzeSummary, name: Spanned<&str>, arg0: &Expr, arg1: &Expr) -> Option<Self> {
                match name.value {
                    $(stringify!($ins) if 2 == instructions!(@count [$($arg),*]) =>
                        Some(instructions!(@args [summary] [Self::$name] [2] [arg0, arg1] [$($arg),*])),
                    )*
                    _ => {
                        summary.error(AnalyzeInsNotExist { location: name.span });
                        None
                    }
                }
            }

            pub fn from_3(summary: &mut AnalyzeSummary, name: Spanned<&str>, arg0: &Expr, arg1: &Expr, arg2: &Expr) -> Option<Self> {
                match name.value {
                    $(stringify!($ins) if 3 == instructions!(@count [$($arg),*]) =>
                        Some(instructions!(@args [summary] [Self::$name] [3] [arg0, arg1, arg2] [$($arg),*])),
                    )*
                    _ => {
                        summary.error(AnalyzeInsNotExist { location: name.span });
                        None
                    }
                }
            }
        }
    };

    ( @count [] ) => {0};
    ( @count [$_:ident] ) => {1};
    ( @count [$_:ident, $($__:tt)*] ) => {1 + instructions!(@count [$($__)*])};

    ( @arg-ty [immediate] ) => {Imm};
    ( @arg-ty [imm] ) => {Imm};
    ( @arg-ty [csr] ) => {Register};
    ( @arg-ty [rs] ) => {Register};
    ( @arg-ty [rt] ) => {Register};
    ( @arg-ty [rd] ) => {Register};
    ( @arg-ty [symbol] ) => {Offset};
    ( @arg-ty [offset] ) => {Offset};

    ( @arg-from [$s:ident] [immediate] [$e:expr] ) => {Imm::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [imm] [$e:expr]) => {Imm::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [csr] [$e:expr]) => {Register::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [rs] [$e:expr]) => {Register::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [rt] [$e:expr]) => {Register::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [rd] [$e:expr]) => {Register::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [symbol] [$e:expr]) => {Offset::from_expr($e, $s)?};
    ( @arg-from [$s:ident] [offset] [$e:expr]) => {Offset::from_expr($e, $s)?};

    ( @arg-mock [immediate] ) => {Imm(0)};
    ( @arg-mock [imm] ) => {Imm(0)};
    ( @arg-mock [csr] ) => {Register::Zero};
    ( @arg-mock [rs] ) => {Register::Zero};
    ( @arg-mock [rt] ) => {Register::Zero};
    ( @arg-mock [rd] ) => {Register::Zero};
    ( @arg-mock [symbol] ) => {Offset::Imm(0)};
    ( @arg-mock [offset] ) => {Offset::Imm(0)};

    ( @arg-offset [$rhs:ident] [$arg:ident] [symbol] ) => {$arg + $rhs.to_owned()};
    ( @arg-offset [$rhs:ident] [$arg:ident] [offset] ) => {$arg + $rhs.to_owned()};
    ( @arg-offset [$rhs:ident] [$arg:ident] [$ty:ident] ) => {$arg};

    ( @args [$s:ident] [$t:expr] [1] [$arg0:ident] [$_:ident] ) => {$t(instructions!(@arg-from [$s] [$_] [$arg0]))};
    ( @args [$s:ident] [$t:expr] [1] [$arg0:ident] [$($tail:ident),*] ) => {
        $t($(instructions!(@arg-mock [$tail])),*)
    };

    ( @args [$s:ident] [$t:expr] [2] [$arg0:ident, $arg1:ident] [$_:ident, $__:ident] ) => {
        $t(instructions!(@arg-from [$s] [$_] [$arg0]), instructions!(@arg-from [$s] [$__] [$arg1]))
    };
    ( @args [$s:ident] [$t:expr] [2] [$arg0:ident, $arg1:ident] [$($tail:ident),*] ) => {
        $t($(instructions!(@arg-mock [$tail])),*)
    };

    ( @args [$s:ident] [$t:expr] [3] [$arg0:ident, $arg1:ident, $arg2:ident] [$_:ident, $__:ident, $___:ident] ) => {
        $t(
            instructions!(@arg-from [$s] [$_] [$arg0]),
            instructions!(@arg-from [$s] [$__] [$arg1]),
            instructions!(@arg-from [$s] [$___] [$arg2])
        )
    };
    ( @args [$s:ident] [$t:expr] [3] [$arg0:ident, $arg1:ident, $arg2:ident] [$($tail:ident),*] ) => {
        $t($(instructions!(@arg-mock [$tail])),*)
    };

    ( @args-indexed [$s:ident] [] ) => {Self::$s()};
    ( @args-indexed [$s:ident] [$_:ident] ) => {Self::$s($_)};
    ( @args-indexed [$s:ident] [$_:ident, $__:ident] ) => {Self::$s($_, $__)};
    ( @args-indexed [$s:ident] [$_:ident, $__:ident, $___:ident] ) => {Self::$s($_, $__, $___)};

    ( @args-offset [$s:ident] [$rhs:ident] [] ) => {Self::$s()};
    ( @args-offset [$s:ident] [$rhs:ident] [$_:ident] ) => {
        Self::$s(instructions!(@arg-offset [$rhs] [$_] [$_]))
    };
    ( @args-offset [$s:ident] [$rhs:ident] [$_:ident, $__:ident] ) => {
        Self::$s(instructions!(@arg-offset [$rhs] [$_] [$_]), instructions!(@arg-offset [$rhs] [$__] [$__]))
    };
    ( @args-offset [$s:ident] [$rhs:ident] [$_:ident, $__:ident, $___:ident] ) => {
        Self::$s(instructions!(@arg-offset [$rhs] [$_] [$_]), instructions!(@arg-offset [$rhs] [$__] [$__]), instructions!(@arg-offset [$rhs] [$___] [$___]))
    };

    ( @args-format [$f:ident] [$s:ident] [] ) => {
        if $f.alternate() {
            $f.write_fmt(format_args!("\x1b[1;38;5;213m{:<4}\x1b[0m", $s))
        } else {
            $f.write_fmt(format_args!("{}", $s))
        }
    };
    ( @args-format [$f:ident] [$s:ident] [$_:ident] ) => {
        if $f.alternate() {
            $f.write_fmt(format_args!("\x1b[1;38;5;211m{:<4}\x1b[0m ", $s))?;
            $_.fmt($f)
        } else {
            $f.write_fmt(format_args!("{} {}", $s, $_))
        }
    };
    ( @args-format [$f:ident] [$s:ident] [$_:ident, $__:ident] ) => {
        if $f.alternate() {
            $f.write_fmt(format_args!("\x1b[1;38;5;177m{:<4}\x1b[0m ", $s))?;
            $_.fmt($f)?;
            $f.write_str(", ")?;
            $__.fmt($f)
        } else {
            $f.write_fmt(format_args!("{} {}, {}", $s, $_, $__))
        }
    };
    ( @args-format [$f:ident] [$s:ident] [$_:ident, $__:ident, $___:ident] ) => {
        if $f.alternate() {
            $f.write_fmt(format_args!("\x1b[1;38;5;176m{:<4}\x1b[0m ", $s))?;
            $_.fmt($f)?;
            $f.write_str(", ")?;
            $__.fmt($f)?;
            $f.write_str(", ")?;
            $___.fmt($f)
        } else {
            $f.write_fmt(format_args!("{} {}, {}, {}", $s, $_, $__, $___))
        }
    };

    ( @aliasing [$s:ident] [($($alias:tt)+)] ) => {stringify!($($alias)+)};
    ( @aliasing [$s:ident] [] ) => {stringify!($s)};
}

instructions! {
[Lui] [lui] [rd, imm] [] [""];
[Auipc] [auipc] [rd, imm] [] [""];
// [Jal] [jal] [] [] [""];
// [Jalr] [jalr] [] [] [""];
[Beq] [beq] [rd, rs, offset] [] [""];
[Bne] [bne] [rd, rs, offset] [] [""];
[Blt] [blt] [rd, rs, offset] [] [""];
[Bge] [bge] [rd, rs, offset] [] [""];
[Bltu] [bltu] [rd, rs, offset] [] [""];
[Bgeu] [bgeu] [rd, rs, offset] [] [""];
[Lb] [lb] [rd, symbol, rs] [] ["Load global"];
[Lh] [lh] [rd, symbol, rs] [] ["Load global"];
[Lw] [lw] [rd, symbol, rs] [] ["Load global"];
[Lbu] [lbu] [rd, rs, offset] [] [""];
[Lhu] [lhu] [rd, rs, offset] [] [""];
// [Sb] [sb] [] [] [""];
// [Sh] [sh] [] [] [""];
// [Sw] [sw] [] [] [""];
[Addi] [addi] [rd, rs, imm] [] [""];
[Slti] [slti] [rd, rs, imm] [] [""];
[Sltiu] [sltiu] [rd, rs, imm] [] [""];
[Xori] [xori] [rd, rs, imm] [] [""];
[Ori] [ori] [rd, rs, imm] [] [""];
[Andi] [andi] [rd, rs, imm] [] [""];
[Slli] [slli] [rd, rs, imm] [] [""];
[Srli] [srli] [rd, rs, imm] [] [""];
[Srai] [srai] [rd, rs, imm] [] [""];
[Add] [add] [rd, rs, rt] [] [""];
[Sub] [sub] [rd, rs, rt] [] [""];
[Sll] [sll] [rd, rs, rt] [] [""];
[Slt] [slt] [rd, rs, rt] [] [""];
[Sltu] [sltu] [rd, rs, rt] [] [""];
[Xor] [xor] [rd, rs, rt] [] [""];
[Srl] [srl] [rd, rs, rt] [] [""];
[Sra] [sra] [rd, rs, rt] [] [""];
[Or] [or] [rd, rs, rt] [] [""];
[And] [and] [rd, rs, rt] [] [""];
[Fence] [fence] [] [] [""];
[FenceI] [fencei (fence.i)] [] [] [""];
[Ecall] [ecall] [] [] [""];
[Ebreak] [ebreak] [] [] [""];
[Csrrw] [csrrw] [] [] [""];
[Csrrs] [csrrs] [] [] [""];
[Csrrc] [csrrc] [] [] [""];
[Csrrwi] [csrrwi] [] [] [""];
[Csrrsi] [csrrsi] [] [] [""];
[Csrrci] [csrrci] [] [] [""];

[Nop] [nop] [] [addi x0, x0, 0] ["No operation"];
[Neg] [neg] [rd, rs] [sub rd, x0, rs] ["Complemento a 2"];
[Negw] [negw] [rd, rs] [subw rd, x0, rs] ["Complemento a 2 (word)"];
[Snez] [snez] [rd, rs] [sltu rd, x0, rs] ["Poner en 1 si 6 = cero"];
[Sltz] [sltz] [rd, rs] [slt rd, rs, x0] ["Poner en 1 si < cero"];
[Sgtz] [sgtz] [rd, rs] [slt rd, x0, rs] ["Poner en 1 si > cero"];
[Beqz] [beqz] [rs, offset] [beq rs, x0, offset] ["Branch si = cero"];
[Bnez] [bnez] [rs, offset] [bne rs, x0, offset] ["Branch si 6 = cero"];
[Blez] [blez] [rs, offset] [bge x0, rs, offset] ["Branch si <= cero"];
[Bgez] [bgez] [rs, offset] [bge rs, x0, offset] ["Branch si >= cero"];
[Bltz] [bltz] [rs, offset] [blt rs, x0, offset] ["Branch si < cero"];
[Bgtz] [bgtz] [rs, offset] [blt x0, rs, offset] ["Branch si > cero"];
[J] [j] [offset] [jal x0, offset] ["Jump"];
[Jr] [jr] [rs] [jalr x0, rs, 0] ["Jump a registro"];
[Ret] [ret] [] [jalr x0, x1, 0] ["Retornar de subrutina"];
[Tail] [tail] [offset] [auipc x6, offset[31:12]] ["Tail call subrutina lejana"];
[Rdinstret] [rdinstret] [] [rd csrrs rd, instret, x0] ["Leer el contador de instrucciones retiradas"];
[RdinstretH] [rdinstreth] [] [rd csrrs rd, instreth, x0] ["Leer el contador de instrucciones retiradas"];
[Rdcycle] [rdcycle] [] [rd csrrs rd, cycle, x0] ["Leer el contador de ciclos"];
[RdcycleH] [rdcycleh] [] [rd csrrs rd, cycleh, x0] ["Leer el contador de ciclos"];
[Rdtime] [rdtime] [] [rd csrrs rd, time, x0] ["Leer real-time clock"];
[RdtimeH] [rdtimeh] [] [rd csrrs rd, timeh, x0] ["Leer real-time clock"];
[Csrr] [csrr] [rd, csr] [csrrs rd, csr, x0] ["Leer CSR"];
[Csrw] [csrw] [csr, rs] [csrrw x0, csr, rs] ["Escribir CSR"];
[Csrs] [csrs] [csr, rs] [csrrs x0, csr, rs] ["Poner bits en 1 en CSR"];
[Csrc] [csrc] [csr, rs] [csrrc x0, csr, rs] ["Poner bits en 0 en CSR"];
[Csrwi] [csrwi] [csr, imm] [csrrwi x0, csr, imm] ["Escribir CSR, inmediato"];
[Csrsi] [csrsi] [csr, imm] [csrrsi x0, csr, imm] ["Poner bits en 1 en CSR, inmediato"];
[Csrci] [csrci] [csr, imm] [csrrci x0, csr, imm] ["Poner bits en 0 en CSR, inmediato"];
[Frcsr] [frcsr] [rd] [csrrs rd, fcsr, x0] ["Leer FP control/status register"];
[Fscsr] [fscsr] [rs] [csrrw x0, fcsr, rs] ["Escribir FP control/status register"];
[Frrm] [frrm] [rd] [csrrs rd, frm, x0] ["Leer FP rounding mode"];
[Fsrm] [fsrm] [rs] [csrrw x0, frm, rs] ["Escribir FP rounding mode"];
[Frflags] [frflags] [rd] [csrrs rd, fflags, x0] ["Leer FP exception flags"];
[Fsflags] [fsflags] [rs] [csrrw x0, fflags, rs] ["Escribir FP exception flags"];

[Lla] [lla] [rd, symbol] [[auipc rd, symbol[31:12]] [addi rd, rd, symbol[11:0]]] ["Load de dirección local"];

[La] [la] [rd, symbol] [[auipc rd, GOT[symbol][31:12]] [lw rd, rd, GOT[symbol][11:0]]] [""];

// [Lb] [lb] [rd, symbol] [[auipc rd, symbol[31:12]] [lb rd, symbol[11:0](rd)]] ["Load global"];
// [Lh] [lh] [rd, symbol] [[auipc rd, symbol[31:12]] [lh rd, symbol[11:0](rd)]] ["Load global"];
// [Lw] [lw] [rd, symbol] [[auipc rd, symbol[31:12]] [lw rd, symbol[11:0](rd)]] ["Load global"];
[Ld] [ld] [rd, symbol] [[auipc rd, symbol[31:12]] [ld rd, symbol[11:0](rd)]] ["Load global"];
[Sb] [sb] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [sb rd, symbol[11:0](rt)]] ["Store global"];
[Sh] [sh] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [sh rd, symbol[11:0](rt)]] ["Store global"];
[Sw] [sw] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [sw rd, symbol[11:0](rt)]] ["Store global"];
[Sd] [sd] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [sd rd, symbol[11:0](rt)]] ["Store global"];
[Flw] [flw] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [flw rd, symbol[11:0](rt)]] ["Load global de punto flotante"];
[Fld] [fld] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [fld rd, symbol[11:0](rt)]] ["Load global de punto flotante"];
[Fsw] [fsw] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [fsw rd, symbol[11:0](rt)]] ["Store global de punto flotante"];
[Fsd] [fsd] [rd, symbol, rt] [[auipc rt, symbol[31:12]] [fsd rd, symbol[11:0](rt)]] ["Store global de punto flotante"];

[Li] [li] [rd, immediate] [Muchas secuencias] ["Load immediate"];
[Mv] [mv] [rd, rs] [addi rd, rs, 0] ["Copiar registro"];
[Not] [not] [rd, rs] [xori rd, rs, -1] ["Complemento a uno"];
[SextW] [sextw (sext.w)] [rd, rs] [addiw rd, rs, 0] ["Sign extend word"];
[Seqz] [seqz] [rd, rs] [sltiu rd, rs, 1] ["Poner en 1 si = cero"];
[FmvS] [fmvs (fmv.s)] [rd, rs] [fsgnj.s rd, rs, rs] ["Copiar registro de precisión simple"];
[FabsS] [fabss (fabs.s)] [rd, rs] [fsgnjx.s rd, rs, rs] ["Valor absoluto de precisión simple"];
[FnegS] [fnegs (fneg.s)] [rd, rs] [fsgnjn.s rd, rs, rs] ["Negación de precisión simple"];
[FmvD] [fmvd (fmv.d)] [rd, rs] [fsgnj.d rd, rs, rs] ["Copiar registro de precisión doble"];
[FabsD] [fabsd (fabs.d)] [rd, rs] [fsgnjx.d rd, rs, rs] ["Valor absoluto de precisión doble"];
[FnegD] [fnegd (fneg.d)] [rd, rs] [fsgnjn.d rd, rs, rs] ["Negación de precisión doble"];
[Bgt] [bgt] [rs, rt, offset] [blt rt, rs, offset] ["Branch si >"];
[Ble] [ble] [rs, rt, offset] [bge rt, rs, offset] ["Branch si <="];
[Bgtu] [bgtu] [rs, rt, offset] [bltu rt, rs, offset] ["Branch si >, unsigned"];
[Bleu] [bleu] [rs, rt, offset] [bgeu rt, rs, offset] ["Branch si <=, unsigned"];
[Jal] [jal] [offset] [jal x1, offset] ["Jump and link"];
[Jalr] [jalr] [rs] [jalr x1, rs, 0] ["Jump and link a registro"];
[Call] [call] [offset] [[auipc x1, offset[31:12]] [jalrx1, x1, offset[11:0]]] ["Llamar subrutina lejana"];
[FenceAll] [fence] [] [fence iorw, iorw] ["Fence en toda la memoria e I/O"];
// [Fscsr] [fscsr] [rd, rs] [csrrw rd, fcsr, rs] ["Swap con FP control/status register"];
// [Fsrm] [fsrm] [rd, rs] [csrrw rd, frm, rs] ["Swap con FP rounding mode"];
// [Fsflags] [fsflags] [rd, rs] [csrrw rd, fflags, rs] ["Swap con FP exception flags"];
}
