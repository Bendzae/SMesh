// use slotmap::KeyData;

// use crate::*;


// unsafe impl slotmap::Key for AttributeKey { fn data(&self) -> slotmap::KeyData {
//         match self {
//             AttributeKey::Vertex(id) => id.data(),
//             AttributeKey::Halfedge(id) => id.data(),
//             AttributeKey::Edge(id) => id.data(),
//             AttributeKey::Face(id) => id.data(),
//         }
//     }
// }
//
// impl From<KeyData>  for AttributeKey{
//     fn from(value: KeyData) -> Self {
//         todo!()
//     }
// }
//
//
// pub trait AttributeValue {
//     fn lerp(&self, other: Self) -> Self;
// }
//
// pub struct AttributeMap<T: AttributeValue> {
//     data: SecondaryMap<AttributeKey, T>,
// }
//
//
// // Test
//
// impl AttributeValue for Vec3 {
//     fn lerp(&self, other: Self) -> Self {
//         todo!()
//     }
// }
