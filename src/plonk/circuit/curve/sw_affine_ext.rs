use super::*;

use crate::bellman::pairing::{
    Engine,
    GenericCurveAffine,
    GenericCurveProjective,
};

use crate::bellman::pairing::ff::{
    Field,
    PrimeField,
    PrimeFieldRepr,
    BitIterator,
    ScalarEngine
};

use crate::bellman::{
    SynthesisError,
};

use crate::bellman::plonk::better_better_cs::cs::{
    Variable, 
    ConstraintSystem,
    ArithmeticTerm,
    MainGateTerm,
    Width4MainGateWithDNext,
    MainGate,
    GateInternal,
    Gate,
    LinearCombinationOfTerms,
    PolynomialMultiplicativeTerm,
    PolynomialInConstraint,
    TimeDilation,
    Coefficient,
    PlonkConstraintSystemParams,
    TrivialAssembly,
    PlonkCsWidth4WithNextStepParams,
};

use crate::plonk::circuit::Assignment;
use crate::plonk::circuit::hashes_with_tables::utils::{IdentifyFirstLast, u64_to_ff};

use super::super::allocated_num::{AllocatedNum, Num};
use super::super::linear_combination::LinearCombination;
use super::super::simple_term::Term;
use super::super::boolean::{Boolean, AllocatedBit};

use plonk::circuit::bigint::*;
use std::convert::From;

// this is ugly and should be rewritten, but OK for initial draft
// it defines elliptic point over Extension Field
#[derive(Clone, Debug)]
pub struct AffinePointExt<'a, E: Engine,  G: GenericCurveAffine, T: Extension2Params<G::Base>> 
where <G as GenericCurveAffine>::Base: PrimeField {
    pub x: Fp2<'a, E, G::Base, T>,
    pub y: Fp2<'a, E, G::Base, T>,
}

impl<'a, E: Engine, G: GenericCurveAffine, T> From<AffinePoint<'a, E, G, T>> for AffinePointExt<'a, E, G, T> 
where <G as GenericCurveAffine>::Base: PrimeField, T: Extension2Params<<G as GenericCurveAffine>::Base>
{
    fn from(item: AffinePoint<'a, E, G, T>) -> Self {
        AffinePointExt::<E, G, T> {
            x: Fp2::from_base_field(item.get_x()),
            y: Fp2::from_base_field(item.get_y())
        } 
    }
}

impl<'a, E: Engine, G: GenericCurveAffine, T> AffinePointExt<'a, E, G, T> 
where <G as GenericCurveAffine>::Base: PrimeField, T: Extension2Params<<G as GenericCurveAffine>::Base> {
    pub fn get_x(&self) -> Fp2<'a, E, G::Base, T> {
        self.x.clone()
    }

    pub fn get_y(&self) -> Fp2<'a, E, G::Base, T> {
        self.y.clone()
    }

    pub fn get_value(&self) -> Option<(G::Base, G::Base, G::Base, G::Base)> {
        self.x.get_value().zip(self.y.get_value()).map(|((x_c0, x_c1), (y_c0, y_c1))| (x_c0, x_c1, y_c0, y_c1) ) 
    }

    pub fn uninitialized(rns_params: &'a RnsParameters<E, G::Base>) -> Self {
        Self::constant(G::Base::zero(), G::Base::zero(), G::Base::zero(), G::Base::zero(), &rns_params)
    }

    #[track_caller]
    pub fn alloc<CS: ConstraintSystem<E>>(
        cs: &mut CS, x_c0_wit: Option<G::Base>, x_c1_wit: Option<G::Base>, 
        y_c0_wit: Option<G::Base>, y_c1_wit: Option<G::Base>,
        rns_params: &'a RnsParameters<E, G::Base>
    ) -> Result<Self, SynthesisError> {
        let x = Fp2::alloc(cs, x_c0_wit, x_c1_wit, rns_params)?;
        let y = Fp2::alloc(cs, y_c0_wit, y_c1_wit, rns_params)?;
        let point = AffinePointExt::<E, G, T> { x, y };
        point.enforce_if_on_curve(cs)?;

        Ok(point)
    } 

    #[track_caller]
    pub fn constant(
        x0: G::Base, x1: G::Base, y0: G::Base, y1: G::Base, rns_params: &'a RnsParameters<E, G::Base>
    ) -> Self {
        let x = Fp2::constant(x0, x1, rns_params);
        let y = Fp2::constant(y0, y1, rns_params);  
        AffinePointExt::<E, G, T> { x, y } 
    }

    #[track_caller]
    pub fn enforce_if_on_curve<CS: ConstraintSystem<E>>(&self, cs: &mut CS) -> Result<(), SynthesisError> {
        let rns_params = self.x.c0.representation_params;
        let a = Fp2::from(FieldElement::constant(G::a_coeff(), rns_params));
        let b = Fp2::from(FieldElement::constant(G::b_coeff(), rns_params));

        let mut lhs = self.y.square(cs)?;
        let x_squared = self.x.square(cs)?;
        let x_cubed = x_squared.mul(cs, &self.x)?;
        let mut rhs = if a.c0.get_field_value().unwrap().is_zero() {
            x_cubed.add(cs, &b)?
        } else {
            let mut chain = Fp2Chain::new();
            chain.add_pos_term(&x_cubed).add_pos_term(&b);
            Fp2::mul_with_chain(cs, &self.x, &a, chain)?
        };

        Fp2::enforce_equal(cs, &mut lhs, &mut rhs)
    }

    #[track_caller]
    pub fn enforce_equal<CS>(cs: &mut CS, left: &mut Self, right: &mut Self) -> Result<(), SynthesisError> 
    where CS: ConstraintSystem<E>
    {
        Fp2::enforce_equal(cs, &mut left.x, &mut right.x)?;
        Fp2::enforce_equal(cs, &mut left.y, &mut right.y)
    }

    #[track_caller]
    pub fn add_unequal_unchecked<CS>(&self, cs: &mut CS, other: &Self) -> Result<Self, SynthesisError> 
    where CS: ConstraintSystem<E>
    {
        match (self.get_value(), other.get_value()) {
            (Some(first), Some(second)) => {
                assert!(first.0 != second.0 || first.1 != second.1, "can't add points with the same x cooridnate");
            },
            _ => {}
        }
        
        let other_x_minus_this_x = other.x.sub(cs, &self.x)?;
        let mut chain = Fp2Chain::new();
        chain.add_pos_term(&other.y).add_neg_term(&self.y);
        let lambda = Fp2::div_with_chain(cs, chain, &other_x_minus_this_x)?;
        
        // lambda^2 + (-x' - x)
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&other.x).add_neg_term(&self.x);
        let new_x = lambda.square_with_chain(cs, chain)?;

        // lambda * (x - new_x) + (- y)
        let this_x_minus_new_x = self.x.sub(cs, &new_x)?;
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.y);
        let new_y = Fp2::mul_with_chain(cs, &lambda, &this_x_minus_new_x, chain)?;

        let new = Self { x: new_x, y: new_y };
        Ok(new)
    }

    #[track_caller]
    pub fn double_and_add_unequal_unchecked<CS>(&self, cs: &mut CS, other: &Self) -> Result<Self, SynthesisError> 
    where CS: ConstraintSystem<E>
    {
        match (self.get_value(), other.get_value()) {
            (Some(first), Some(second)) => {
                assert!(first.0 != second.0 || first.1 != second.1, "can't add points with the same x cooridnate");
            },
            _ => {}
        }
        
        let other_x_minus_this_x = other.x.sub(cs, &self.x)?;
        let mut chain = Fp2Chain::new();
        chain.add_pos_term(&other.y).add_neg_term(&self.y); 
        let lambda = Fp2::div_with_chain(cs, chain, &other_x_minus_this_x)?;

        // lambda^2 + (-x' - x)
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&other.x).add_neg_term(&self.x);
        let new_x = lambda.square_with_chain(cs, chain)?;
        
        let new_x_minus_this_x = new_x.sub(cs, &self.x)?;
        let two_y = self.y.double(cs)?;
        let t0 = two_y.div(cs, &new_x_minus_this_x)?;
        let t1 = lambda.add(cs, &t0)?;
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.x).add_neg_term(&new_x);
        let new_x = t1.square_with_chain(cs, chain)?;

        let new_x_minus_x= new_x.sub(cs, &self.x)?;
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.y);
        let new_y = Fp2::mul_with_chain(cs, &t1, &new_x_minus_x, chain)?;

        let new = Self { x: new_x, y: new_y };
        Ok(new)
    }

    #[track_caller]
    pub fn sub_unequal_unchecked<CS>(&self, cs: &mut CS, other: &Self) -> Result<Self, SynthesisError> 
    where CS: ConstraintSystem<E>
    {
        match (self.get_value(), other.get_value()) {
            (Some(first), Some(second)) => {
                assert!(first.0 != second.0 || first.1 != second.1, "can't add points with the same x cooridnate");
            },
            _ => {}
        }
        
        let other_x_minus_this_x = other.x.sub(cs, &self.x)?;
        let mut chain = Fp2Chain::new();
        chain.add_pos_term(&other.y).add_pos_term(&self.y);
        let lambda = Fp2::div_with_chain(cs, chain, &other_x_minus_this_x)?;

        // lambda^2 + (-x' - x)
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.x).add_neg_term(&other.x);
        let new_x = lambda.square_with_chain(cs, chain)?;

        // lambda * -(x - new_x) + (- y)
        let new_x_minus_this_x = new_x.sub(cs, &self.x)?;
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.y);
        let new_y = Fp2::mul_with_chain(cs, &lambda, &new_x_minus_this_x, chain)?;

        let new = Self { x: new_x, y: new_y};
        Ok(new)
    }

    #[track_caller]
    pub fn double<CS: ConstraintSystem<E>>(&self, cs: &mut CS) -> Result<Self, SynthesisError> {
        let x_squared = self.x.square(cs)?;
        let mut chain = Fp2Chain::new();
        chain.add_pos_term(&x_squared).add_pos_term(&x_squared).add_pos_term(&x_squared);
        let two_y = self.y.double(cs)?;
        let lambda = Fp2::div_with_chain(cs, chain, &two_y)?;

        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.x).add_neg_term(&self.x);
        let new_x = lambda.square_with_chain(cs, chain)?;

        let x_minus_new_x = self.x.sub(cs, &new_x)?;
        let mut chain = Fp2Chain::new();
        chain.add_neg_term(&self.y);
        let new_y = Fp2::mul_with_chain(cs, &lambda, &x_minus_new_x, chain)?;

        let new = Self { x: new_x, y: new_y };
        Ok(new)
    }

    #[track_caller]
    pub fn conditionally_select<CS: ConstraintSystem<E>>(
        cs: &mut CS, flag: &Boolean, first: &Self, second: &Self
    ) -> Result<Self, SynthesisError> {
        let x = Fp2::conditionally_select(cs, &flag, &first.x, &second.x)?;
        let y = Fp2::conditionally_select(cs, &flag, &first.y, &second.y)?;
        Ok(AffinePointExt {x, y})
    }

    #[track_caller]
    pub fn negate<CS: ConstraintSystem<E>>(&self, cs: &mut CS) -> Result<Self, SynthesisError> {
        let x = self.x.clone();
        let y = self.y.negate(cs)?;
        Ok(AffinePointExt {x, y})
    }

    #[track_caller]
    pub fn conditionally_negate<CS>(&self, cs: &mut CS, flag: &Boolean) -> Result<Self, SynthesisError> 
    where CS: ConstraintSystem<E> 
    {
        let x = self.x.clone();
        let y = self.y.conditionally_negate(cs, flag)?;
        Ok(AffinePointExt {x, y})
    }

    pub fn mixed_add_unequal_unchecked<CS: ConstraintSystem<E>>(
        &self, cs: &mut CS, elem: &AffinePoint<'a, E, G, T>
    ) -> Result<Self, SynthesisError> {
        match (self.get_value(), elem.get_value()) {
            (Some(first), Some(second)) => {
                let (second_x, _second_y) = second.into_xy_unchecked();
                assert!(
                    first.0 != second_x || first.1 != G::Base::zero(), 
                    "can't add points with the same x cooridnate"
                );
            },
            _ => {}
        }

        let elem_ext = Self::from(elem.clone());
        self.add_unequal_unchecked(cs, &elem_ext)
    }

    pub fn mixed_sub_unequal_unchecked<CS: ConstraintSystem<E>>(
        &self, cs: &mut CS, elem: &AffinePoint<'a, E, G, T>
    ) -> Result<Self, SynthesisError> {
        match (self.get_value(), elem.get_value()) {
            (Some(first), Some(second)) => {
                let (second_x, _second_y) = second.into_xy_unchecked();
                assert!(
                    first.0 != second_x || first.1 != G::Base::zero(), 
                    "can't add points with the same x cooridnate"
                );
            },
            _ => {}
        }

        let elem_ext = Self::from(elem.clone());
        self.sub_unequal_unchecked(cs, &elem_ext)
    }

    #[track_caller]
    pub fn mixed_double_and_add_unequal_unchecked<CS: ConstraintSystem<E>>(
        &self, cs: &mut CS, elem: &AffinePoint<'a, E, G, T>
    ) -> Result<Self, SynthesisError> {
        match (self.get_value(), elem.get_value()) {
            (Some(first), Some(second)) => {
                let (second_x, _second_y) = second.into_xy_unchecked();
                assert!(
                    first.0 != second_x || first.1 != G::Base::zero(), 
                    "can't add points with the same x cooridnate"
                );
            },
            _ => {}
        }

        let elem_ext = Self::from(elem.clone());
        self.double_and_add_unequal_unchecked(cs, &elem_ext)
    }
}