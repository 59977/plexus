use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};

use geometry::Geometry;
use graph::mesh::Mesh;
use graph::storage::convert::{AsStorage, AsStorageMut};
use graph::storage::{Bind, EdgeKey, FaceKey, VertexKey};
use graph::topology::{Edge, Face, Topological, Vertex};
use graph::view::convert::{FromKeyedSource, IntoView};
use graph::view::{
    Consistency, Consistent, EdgeView, FaceView, Inconsistent, OrphanEdgeView, OrphanFaceView,
};
use BoolExt;

/// Do **not** use this type directly. Use `VertexRef` and `VertexMut` instead.
///
/// This type is only re-exported so that its members are shown in
/// documentation. See this issue:
/// <https://github.com/rust-lang/rust/issues/39437>
pub struct VertexView<M, G, C = Inconsistent>
where
    M: AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    key: VertexKey,
    storage: M,
    phantom: PhantomData<(G, C)>,
}

/// Storage.
impl<M, G, C> VertexView<M, G, C>
where
    M: AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    pub(in graph) fn bind<T, N>(self, storage: N) -> VertexView<<M as Bind<T, N>>::Output, G, C>
    where
        T: Topological,
        M: Bind<T, N>,
        M::Output: AsStorage<Vertex<G>>,
        N: AsStorage<T>,
    {
        let (key, origin) = self.into_keyed_storage();
        VertexView::from_keyed_storage_unchecked(key, origin.bind(storage))
    }
}

impl<'a, M, G, C> VertexView<&'a mut M, G, C>
where
    M: 'a + AsStorage<Vertex<G>> + AsStorageMut<Vertex<G>>,
    G: 'a + Geometry,
    C: Consistency,
{
    pub fn into_orphan(self) -> OrphanVertexView<'a, G> {
        let (key, storage) = self.into_keyed_storage();
        (key, storage.as_storage_mut().get_mut(&key).unwrap())
            .into_view()
            .unwrap()
    }
}

impl<'a, M, G> VertexView<&'a mut M, G, Consistent>
where
    M: 'a + AsRef<Mesh<G>> + AsMut<Mesh<G>> + AsStorage<Vertex<G>> + AsStorageMut<Vertex<G>>,
    G: 'a + Geometry,
{
    pub fn into_ref(self) -> VertexView<&'a M, G, Consistent> {
        let (key, storage) = self.into_keyed_storage();
        VertexView::<_, _, Consistent>::from_keyed_storage(key, &*storage).unwrap()
    }
}

impl<M, G, C> VertexView<M, G, C>
where
    M: AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    pub fn key(&self) -> VertexKey {
        self.key
    }

    pub(in graph) fn from_keyed_storage(key: VertexKey, storage: M) -> Option<Self> {
        storage
            .as_storage()
            .contains_key(&key)
            .into_some(VertexView::from_keyed_storage_unchecked(key, storage))
    }

    fn from_keyed_storage_unchecked(key: VertexKey, storage: M) -> Self {
        VertexView {
            key,
            storage,
            phantom: PhantomData,
        }
    }

    pub(in graph) fn into_keyed_storage(self) -> (VertexKey, M) {
        let VertexView { key, storage, .. } = self;
        (key, storage)
    }

    fn interior_ref(&self) -> VertexView<&M, G, C> {
        let key = self.key;
        let storage = &self.storage;
        VertexView::from_keyed_storage_unchecked(key, storage)
    }
}

/// Reachable API.
impl<M, G, C> VertexView<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    pub(in graph) fn into_reachable_outgoing_edge(self) -> Option<EdgeView<M, G, C>> {
        let key = self.edge;
        key.and_then(move |key| {
            let (_, storage) = self.into_keyed_storage();
            EdgeView::<_, _, C>::from_keyed_storage(key, storage)
        })
    }

    pub(in graph) fn reachable_outgoing_edge(&self) -> Option<EdgeView<&M, G, C>> {
        self.edge.and_then(|key| {
            let storage = &self.storage;
            EdgeView::<_, _, C>::from_keyed_storage(key, storage)
        })
    }

    pub(in graph) fn reachable_incoming_edges(&self) -> EdgeCirculator<&M, G, C> {
        let key = self.edge;
        let storage = &self.storage;
        EdgeCirculator::from_keyed_storage(key, storage)
    }
}

impl<M, G> VertexView<M, G, Consistent>
where
    M: AsRef<Mesh<G>> + AsStorage<Edge<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
{
    pub fn into_outgoing_edge(self) -> EdgeView<M, G, Consistent> {
        self.into_reachable_outgoing_edge().unwrap()
    }

    pub fn outgoing_edge(&self) -> EdgeView<&Mesh<G>, G, Consistent> {
        interior_deref!(edge => self.reachable_outgoing_edge().unwrap())
    }
}

impl<M, G> VertexView<M, G, Consistent>
where
    M: AsRef<Mesh<G>> + AsStorage<Edge<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
{
    pub fn incoming_edges(&self) -> EdgeCirculator<&Mesh<G>, G, Consistent> {
        let key = self.edge;
        let storage = self.storage.as_ref();
        EdgeCirculator::from_keyed_storage(key, storage)
    }
}

/// Reachable API.
impl<M, G, C> VertexView<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Face<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    pub(in graph) fn reachable_neighboring_faces(&self) -> FaceCirculator<&M, G, C> {
        FaceCirculator::from(EdgeCirculator::from(self.interior_ref()))
    }
}

impl<M, G> VertexView<M, G, Consistent>
where
    M: AsRef<Mesh<G>> + AsStorage<Edge<G>> + AsStorage<Face<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
{
    pub fn neighboring_faces(&self) -> FaceCirculator<&Mesh<G>, G, Consistent> {
        let key = self.edge;
        let storage = self.storage.as_ref();
        FaceCirculator::from(EdgeCirculator::from_keyed_storage(key, storage))
    }
}

/// Reachable API.
impl<M, G, C> VertexView<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorageMut<Edge<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    pub(in graph) fn reachable_outgoing_orphan_edge(&mut self) -> Option<OrphanEdgeView<G>> {
        if let Some(key) = self.edge {
            Some(
                (key, self.storage.as_storage_mut().get_mut(&key).unwrap())
                    .into_view()
                    .unwrap(),
            )
        }
        else {
            None
        }
    }

    pub(in graph) fn reachable_incoming_orphan_edges(&mut self) -> EdgeCirculator<&mut M, G, C> {
        let key = self.edge;
        let storage = &mut self.storage;
        EdgeCirculator::from_keyed_storage(key, storage)
    }
}

impl<M, G> VertexView<M, G, Consistent>
where
    M: AsRef<Mesh<G>>
        + AsMut<Mesh<G>>
        + AsStorage<Edge<G>>
        + AsStorageMut<Edge<G>>
        + AsStorage<Vertex<G>>,
    G: Geometry,
{
    pub fn outgoing_orphan_edge(&mut self) -> OrphanEdgeView<G> {
        self.reachable_outgoing_orphan_edge().unwrap()
    }

    pub fn incoming_orphan_edges(&mut self) -> EdgeCirculator<&mut Mesh<G>, G, Consistent> {
        let key = self.edge;
        let storage = self.storage.as_mut();
        EdgeCirculator::from_keyed_storage(key, storage)
    }
}

/// Reachable API.
impl<M, G, C> VertexView<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Face<G>> + AsStorageMut<Face<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    pub(in graph) fn reachable_neighboring_orphan_faces(&mut self) -> FaceCirculator<&mut M, G, C> {
        let key = self.edge;
        let storage = &mut self.storage;
        FaceCirculator::from(EdgeCirculator::from_keyed_storage(key, storage))
    }
}

impl<M, G> VertexView<M, G, Consistent>
where
    M: AsRef<Mesh<G>>
        + AsMut<Mesh<G>>
        + AsStorage<Edge<G>>
        + AsStorage<Face<G>>
        + AsStorageMut<Face<G>>
        + AsStorage<Vertex<G>>,
    G: Geometry,
{
    pub fn neighboring_orphan_faces(&mut self) -> FaceCirculator<&mut Mesh<G>, G, Consistent> {
        let key = self.edge;
        let storage = self.storage.as_mut();
        FaceCirculator::from(EdgeCirculator::from_keyed_storage(key, storage))
    }
}

impl<M, G, C> Clone for VertexView<M, G, C>
where
    M: AsStorage<Vertex<G>> + Clone,
    G: Geometry,
    C: Consistency,
{
    fn clone(&self) -> Self {
        VertexView {
            key: self.key,
            storage: self.storage.clone(),
            phantom: PhantomData,
        }
    }
}

impl<M, G, C> Copy for VertexView<M, G, C>
where
    M: AsStorage<Vertex<G>> + Copy,
    G: Geometry,
    C: Consistency,
{}

impl<M, G, C> Deref for VertexView<M, G, C>
where
    M: AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    type Target = Vertex<G>;

    fn deref(&self) -> &Self::Target {
        self.storage.as_storage().get(&self.key).unwrap()
    }
}

impl<M, G, C> DerefMut for VertexView<M, G, C>
where
    M: AsStorage<Vertex<G>> + AsStorageMut<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.storage.as_storage_mut().get_mut(&self.key).unwrap()
    }
}

impl<M, G> FromKeyedSource<(VertexKey, M)> for VertexView<M, G, Inconsistent>
where
    M: AsStorage<Vertex<G>>,
    G: Geometry,
{
    fn from_keyed_source(source: (VertexKey, M)) -> Option<Self> {
        let (key, storage) = source;
        VertexView::<_, _, Inconsistent>::from_keyed_storage(key, storage)
    }
}

impl<M, G> FromKeyedSource<(VertexKey, M)> for VertexView<M, G, Consistent>
where
    M: AsRef<Mesh<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
{
    fn from_keyed_source(source: (VertexKey, M)) -> Option<Self> {
        let (key, storage) = source;
        VertexView::<_, _, Consistent>::from_keyed_storage(key, storage)
    }
}

/// Do **not** use this type directly. Use `OrphanVertex` instead.
///
/// This type is only re-exported so that its members are shown in
/// documentation. See this issue:
/// <https://github.com/rust-lang/rust/issues/39437>
pub struct OrphanVertexView<'a, G>
where
    G: 'a + Geometry,
{
    key: VertexKey,
    vertex: &'a mut Vertex<G>,
}

impl<'a, G> OrphanVertexView<'a, G>
where
    G: 'a + Geometry,
{
    fn from_keyed_storage(key: VertexKey, vertex: &'a mut Vertex<G>) -> Self {
        OrphanVertexView { key, vertex }
    }

    pub fn key(&self) -> VertexKey {
        self.key
    }
}

impl<'a, G> Deref for OrphanVertexView<'a, G>
where
    G: 'a + Geometry,
{
    type Target = Vertex<G>;

    fn deref(&self) -> &Self::Target {
        &*self.vertex
    }
}

impl<'a, G> DerefMut for OrphanVertexView<'a, G>
where
    G: 'a + Geometry,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.vertex
    }
}

impl<'a, G> FromKeyedSource<(VertexKey, &'a mut Vertex<G>)> for OrphanVertexView<'a, G>
where
    G: 'a + Geometry,
{
    fn from_keyed_source(source: (VertexKey, &'a mut Vertex<G>)) -> Option<Self> {
        let (key, vertex) = source;
        Some(OrphanVertexView::from_keyed_storage(key, vertex))
    }
}

pub struct EdgeCirculator<M, G, C>
where
    M: AsStorage<Edge<G>>,
    G: Geometry,
    C: Consistency,
{
    storage: M,
    outgoing: Option<EdgeKey>,
    breadcrumb: Option<EdgeKey>,
    phantom: PhantomData<(G, C)>,
}

impl<M, G, C> EdgeCirculator<M, G, C>
where
    M: AsStorage<Edge<G>>,
    G: Geometry,
    C: Consistency,
{
    fn from_keyed_storage(key: Option<EdgeKey>, storage: M) -> Self {
        EdgeCirculator {
            storage,
            outgoing: key,
            breadcrumb: key,
            phantom: PhantomData,
        }
    }

    fn next(&mut self) -> Option<EdgeKey> {
        self.outgoing
            .and_then(|outgoing| self.storage.as_storage().get(&outgoing))
            .and_then(|outgoing| outgoing.opposite)
            .and_then(|incoming| {
                self.storage
                    .as_storage()
                    .get(&incoming)
                    .map(|incoming| incoming.next)
                    .map(|outgoing| (incoming, outgoing))
            })
            .and_then(|(incoming, outgoing)| {
                self.breadcrumb.map(|_| {
                    if self.breadcrumb == outgoing {
                        self.breadcrumb = None;
                    }
                    else {
                        self.outgoing = outgoing;
                    }
                    incoming
                })
            })
    }
}

impl<M, G, C> From<VertexView<M, G, C>> for EdgeCirculator<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Vertex<G>>,
    G: Geometry,
    C: Consistency,
{
    fn from(vertex: VertexView<M, G, C>) -> Self {
        let key = vertex.edge;
        let (_, storage) = vertex.into_keyed_storage();
        EdgeCirculator {
            storage,
            outgoing: key,
            breadcrumb: key,
            phantom: PhantomData,
        }
    }
}

impl<'a, M, G, C> Iterator for EdgeCirculator<&'a M, G, C>
where
    M: 'a + AsStorage<Edge<G>>,
    G: 'a + Geometry,
    C: Consistency,
{
    type Item = EdgeView<&'a M, G, C>;

    fn next(&mut self) -> Option<Self::Item> {
        EdgeCirculator::<&'a M, G, C>::next(self)
            .and_then(|key| EdgeView::from_keyed_storage(key, self.storage))
    }
}

impl<'a, M, G, C> Iterator for EdgeCirculator<&'a mut M, G, C>
where
    M: 'a + AsStorage<Edge<G>> + AsStorageMut<Edge<G>>,
    G: 'a + Geometry,
    C: Consistency,
{
    type Item = OrphanEdgeView<'a, G>;

    fn next(&mut self) -> Option<Self::Item> {
        EdgeCirculator::<&'a mut M, G, C>::next(self).and_then(|key| {
            (key, unsafe {
                // Apply `'a` to the autoref from `as_storage_mut` and
                // `get_mut`.
                mem::transmute::<&'_ mut Edge<G>, &'a mut Edge<G>>(
                    self.storage.as_storage_mut().get_mut(&key).unwrap(),
                )
            }).into_view()
        })
    }
}

pub struct FaceCirculator<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Face<G>>,
    G: Geometry,
    C: Consistency,
{
    input: EdgeCirculator<M, G, C>,
}

impl<M, G, C> FaceCirculator<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Face<G>>,
    G: Geometry,
    C: Consistency,
{
    fn next(&mut self) -> Option<FaceKey> {
        while let Some(edge) = self.input.next() {
            if let Some(face) = AsStorage::<Edge<G>>::as_storage(&self.input.storage)
                .get(&edge)
                .and_then(|edge| edge.face)
            {
                return Some(face);
            }
            else {
                // Skip edges with no face. This can occur within non-enclosed
                // meshes.
                continue;
            }
        }
        None
    }
}

impl<M, G, C> From<EdgeCirculator<M, G, C>> for FaceCirculator<M, G, C>
where
    M: AsStorage<Edge<G>> + AsStorage<Face<G>>,
    G: Geometry,
    C: Consistency,
{
    fn from(input: EdgeCirculator<M, G, C>) -> Self {
        FaceCirculator { input }
    }
}

impl<'a, M, G, C> Iterator for FaceCirculator<&'a M, G, C>
where
    M: 'a + AsStorage<Edge<G>> + AsStorage<Face<G>>,
    G: 'a + Geometry,
    C: Consistency,
{
    type Item = FaceView<&'a M, G, C>;

    fn next(&mut self) -> Option<Self::Item> {
        FaceCirculator::<&'a M, G, C>::next(self)
            .and_then(|key| FaceView::from_keyed_storage(key, self.input.storage))
    }
}

impl<'a, M, G, C> Iterator for FaceCirculator<&'a mut M, G, C>
where
    M: 'a + AsStorage<Edge<G>> + AsStorage<Face<G>> + AsStorageMut<Face<G>>,
    G: 'a + Geometry,
    C: Consistency,
{
    type Item = OrphanFaceView<'a, G>;

    fn next(&mut self) -> Option<Self::Item> {
        FaceCirculator::<&'a mut M, G, C>::next(self).and_then(|key| {
            (key, unsafe {
                // Apply `'a` to the autoref from `as_storage_mut` and
                // `get_mut`.
                mem::transmute::<&'_ mut Face<G>, &'a mut Face<G>>(
                    self.input.storage.as_storage_mut().get_mut(&key).unwrap(),
                )
            }).into_view()
        })
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::Point3;

    use generate::*;
    use graph::*;

    #[test]
    fn circulate_over_edges() {
        let mesh = sphere::UvSphere::new(4, 2)
            .polygons_with_position() // 6 triangles, 18 vertices.
            .collect::<Mesh<Point3<f32>>>();

        // All faces should be triangles and all vertices should have 4
        // (incoming) edges.
        for vertex in mesh.vertices() {
            assert_eq!(4, vertex.incoming_edges().count());
        }
    }
}
