use crate::bellman::plonk::better_better_cs::cs::*;
use crate::bellman::plonk::better_better_cs::lookup_tables::*;
use crate::bellman::plonk::better_better_cs::utils;
use crate::bellman::pairing::ff::*;
use crate::bellman::SynthesisError;
use crate::bellman::Engine;
use crate::plonk::circuit::bigint_new::fe_to_biguint;
use bellman::GenericCurveAffine;
use plonk::circuit::bigint::RnsParameters;
use plonk::circuit::curve::vec_of_bit;
use crate::bellman::pairing::ff::{
    Field,
    PrimeField,
    PrimeFieldRepr,
    BitIterator,
    ScalarEngine
};
use bellman::bn256::Fr;
use bellman::GenericCurveProjective;
use plonk::circuit::bigint::FieldElement;

use itertools::Itertools;

use super::AffinePoint;
// A table for storing a AffinePoint from a generator.
// Create a table of the view:
// _________________________________________________
// |  scalar || flag | limb_low_x_0 | limb_low_x_1 |
// |  scalar || flag | limb_high_x_0| limb_high_x_1|
// |    .   .   .   .   .   .   .   .   .   .   .  |
// _________________________________________________
// __________________________________________________
// |  scalar || flag | limb_low_y_0  | limb_low_y_1 |           
// |  scalar || flag | limb_high_y_0 | limb_high_y_1|
// |    .   .   .   .   .   .   .   .   .   .    .  |
// __________________________________________________
#[derive(Clone)]
pub struct ScalarPointTable<E: Engine>{
    table_entries: [Vec<E::Fr>; 3],
    table_lookup_map: std::collections::HashMap<E::Fr, (E::Fr, E::Fr)>,
    table_len: usize, 
    name: &'static str,
}

impl<E: Engine> ScalarPointTable<E>{
    pub fn new_x_table<'a, F: PrimeField, G: GenericCurveAffine<Base = F>>(window: usize, name: &'static str, params: &'a RnsParameters<E, F>) -> Self{
        // there will be exactly as many points as the characteristics of the field
        // multiplied by 2, because 1 wheelbarrow occupies 2 cells
        let bit_window = (2 as u64).pow(window as u32) as usize;
        let table_len = (bit_window * 2) as usize;
        // column0 is our key scalar || flag
        let mut column0 = Vec::with_capacity(table_len);
        let mut column1 = Vec::with_capacity(table_len);
        let mut column2 = Vec::with_capacity(table_len);
        let mut map = std::collections::HashMap::with_capacity(table_len);

        let offset_generator = G::one();



        for i in 0..bit_window{
            // for the key we calculate a constant in the binary representation. 
            // However, we will count the scalar for the point in the skew representation
            // Example: 0 1 01 11 100       if  window-3 000, 001, 011  --- bin rep
            // Example: number3 –– 011 ------ 1  skew 111 -7       

            // this scalar_num calculate for the constant by which we will multiply the point
            let (_, scalar_num) = vec_of_bit(i, window);
            let unsign_nuber = i64::abs(scalar_num);
            // 0 || scalar
            let scalar_x_low = E::Fr::from_str(&format!("{}", (i*2))).unwrap(); 
            // 1 || scalar
            let scalar_x_high = E::Fr::from_str(&format!("{}", (i*2+1))).unwrap();

            column0.push(scalar_x_low);
            column0.push(scalar_x_high);


            let scalar = G::Scalar::from_str(&format!("{}", unsign_nuber)).unwrap();
            // n*G
            let point = offset_generator.mul(scalar);
            let generator = AffinePoint::constant(point.into_affine(), params);

            let limbs = FieldElement::into_limbs(generator.x.clone());
            // low_limb
            column1.push(limbs[0].get_value().unwrap());
            column2.push(limbs[1].get_value().unwrap());
            // high_limb
            column1.push(limbs[2].get_value().unwrap());
            column2.push(limbs[3].get_value().unwrap());

            map.insert(scalar_x_low, (limbs[0].get_value().unwrap(), limbs[1].get_value().unwrap()));
            map.insert(scalar_x_low, (limbs[2].get_value().unwrap(), limbs[3].get_value().unwrap()));


        }

        Self { 
            table_entries: [column0, column1, column2],
            table_lookup_map: map, 
            table_len,
            name
        }

    }
    pub fn new_y_table<'a, F: PrimeField, G: GenericCurveAffine<Base = F>>(window: usize, name: &'static str, params: &'a RnsParameters<E, F>) -> Self{
        // there will be exactly as many points as the characteristics of the field
        // multiplied by 2, because 1 wheelbarrow occupies 2 cells
        let bit_window = (2 as u64).pow(window as u32) as usize;
        let table_len = (bit_window * 2) as usize;
        // column0 is our key scalar || flag
        let mut column0 = Vec::with_capacity(table_len);
        let mut column1 = Vec::with_capacity(table_len);
        let mut column2 = Vec::with_capacity(table_len);
        let mut map = std::collections::HashMap::with_capacity(table_len);

        let offset_generator = G::one();
        // let point = offset_generator.into_projective();


        for i in 0..bit_window{
            // for the key we calculate a constant in the binary representation. 
            // However, we will count the scalar for the point in the skew representation
            // Example: 0 1 01 11 100       if  window-3 000, 001, 011  --- bin rep
            // Example: number3 –– 011 ------ 1  skew 111 -7       

            // this scalar_num calculate for the constant by which we will multiply the point
            let (_, scalar_num) = vec_of_bit(i, window);
            // sigh of scalar
            let a = i64::abs(scalar_num);
            let diff = scalar_num - a;
            let unsign_nuber = i64::abs(scalar_num);
            // 0 || scalar
            let scalar_y_low = E::Fr::from_str(&format!("{}", (i*2))).unwrap(); 
            // 1 || scalar
            let scalar_y_high = E::Fr::from_str(&format!("{}", (i*2+1))).unwrap();

            column0.push(scalar_y_low);
            column0.push(scalar_y_high);


            let scalar = G::Scalar::from_str(&format!("{}", unsign_nuber)).unwrap();
            // n*G
            let mut point = offset_generator.mul(scalar);
            if diff == 0{
                point.negate();
            }
            let generator = AffinePoint::constant(point.into_affine(), params);

            let limbs = FieldElement::into_limbs(generator.y.clone());
            // low_limb
            column1.push(limbs[0].get_value().unwrap());
            column2.push(limbs[1].get_value().unwrap());
            // high_limb
            column1.push(limbs[2].get_value().unwrap());
            column2.push(limbs[3].get_value().unwrap());

            map.insert(scalar_y_low, (limbs[0].get_value().unwrap(), limbs[1].get_value().unwrap()));
            map.insert(scalar_y_low, (limbs[2].get_value().unwrap(), limbs[3].get_value().unwrap()));


        }

        Self { 
            table_entries: [column0, column1, column2],
            table_lookup_map: map, 
            table_len,
            name
        }

    }
}

impl<E: Engine> std::fmt::Debug for ScalarPointTable<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarPointTable").finish()
    }
}
impl<E: Engine> LookupTableInternal<E> for ScalarPointTable<E> {
    fn name(&self) -> &'static str {
        self.name
    }
    fn table_size(&self) -> usize {
        self.table_len
    }
    fn num_keys(&self) -> usize {
        1
    }
    fn num_values(&self) -> usize {
        2
    }
    fn allows_combining(&self) -> bool {
        true
    }
    fn get_table_values_for_polys(&self) -> Vec<Vec<E::Fr>> {
        vec![self.table_entries[0].clone(), self.table_entries[1].clone(), self.table_entries[2].clone()]
    }
    fn table_id(&self) -> E::Fr {
        table_id_from_string(self.name)
    }
    fn sort(&self, _values: &[E::Fr], _column: usize) -> Result<Vec<E::Fr>, SynthesisError> {
        unimplemented!()
    }
    fn box_clone(&self) -> Box<dyn LookupTableInternal<E>> {
        Box::from(self.clone())
    }
    fn column_is_trivial(&self, column_num: usize) -> bool {
        false
    }

    fn is_valid_entry(&self, keys: &[E::Fr], values: &[E::Fr]) -> bool {
        assert!(keys.len() == self.num_keys());
        assert!(values.len() == self.num_values());

        if let Some(entry) = self.table_lookup_map.get(&keys[0]) {
            return entry == &(values[0], values[1]);
        }
        false
    }

    fn query(&self, keys: &[E::Fr]) -> Result<Vec<E::Fr>, SynthesisError> {
        assert!(keys.len() == self.num_keys());

        if let Some(entry) = self.table_lookup_map.get(&keys[0]) {
            return Ok(vec![entry.0, entry.1])
        }

        Err(SynthesisError::Unsatisfiable)
    }
}