use core::slice;

use crate::ExecutionContext;
use crate::data_cell::DataCell;
use crate::data_cell::DataCellOps;
use crate::data_cell::Error;
use crate::data_cell::expr::Expr;
use crate::data_cell::expr::PostfixExpr;
use crate::data_cell::expr::PostfixRoot;
use crate::data_cell::expr::PostfixItem;
use crate::data_cell::expr::PrimaryExpr;
use crate::log_debug;

pub trait Eval {
    fn eval_with_cell_stack<'x>(
        &self,
        _cell_stack: &mut[DataCell<'x>],
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>>;

    fn eval_on_cell<'x>(
        &self,
        cell: &mut DataCell<'x>,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        self.eval_with_cell_stack(slice::from_mut(cell), xc)
    }
}

impl Eval for PrimaryExpr<'_> {
    fn eval_with_cell_stack<'x>(
        &self,
        cell_stack: &mut[DataCell<'x>],
        xc: &mut ExecutionContext<'x>
    ) -> Result<DataCell<'x>, Error<'x>> {
        match self {
            PrimaryExpr::Identifier(s) => {
                let s = s.as_str();
                for c in cell_stack.rchunks_exact_mut(1) {
                    let c = &mut c[0];
                    log_debug!(xc, "querying {:?} for attr {:?}", c, s);
                    match c.get_property(s, xc) {
                        Ok(v) => {
                            return Ok(v);
                        },
                        Err(e) => {
                            if e != Error::NotApplicable {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(Error::NotApplicable)
            },
        }
    }
}

impl Eval for PostfixRoot<'_> {
    fn eval_with_cell_stack<'x>(
        &self,
        cell_stack: &mut[DataCell<'x>],
        xc: &mut ExecutionContext<'x>
    ) -> Result<DataCell<'x>, Error<'x>> {
        match self {
            PostfixRoot::Primary(pe) => pe.eval_with_cell_stack(cell_stack, xc),
        }
    }
}

impl Eval for PostfixExpr<'_> {
    fn eval_with_cell_stack<'x>(
        &self,
        cell_stack: &mut[DataCell<'x>],
        xc: &mut ExecutionContext<'x>
    ) -> Result<DataCell<'x>, Error<'x>> {
        let mut v = self.root.eval_with_cell_stack(cell_stack, xc)?;
        for pfi in self.items.as_slice() {
            v = match pfi {
                PostfixItem::Property(p) => v.get_property(p.as_str(), xc)?
            };
        }
        Ok(v)
    }
}

impl Eval for Expr<'_> {
    fn eval_with_cell_stack<'x>(
        &self,
        cell_stack: &mut[DataCell<'x>],
        xc: &mut ExecutionContext<'x>
    ) -> Result<DataCell<'x>, Error<'x>> {
        match self {
            Expr::Postfix(pfe) => pfe.eval_with_cell_stack(cell_stack, xc),
        }
    }
}

