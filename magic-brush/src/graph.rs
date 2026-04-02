use crate::utils::FromNormalized;

/// A 2D graph that maps input to output.
/// 
/// Currently, there is only a single trait implementation for `[[f32; 2]]`, which is a slice of points in the graph.
/// The first value is for input and the second value is for output. Please note that the content of the slice must be
/// sorted by input values, as the implementation does not check to ensure the content is sorted before sampling values.
pub trait Graph {
    /// Sample a value from the graph.
    fn sample_graph(&self, input: f32) -> f32;

    /// Make 1D texture data for binding lookup table to shader.
    fn make_1d_data<T, const N: usize>(&self) -> [T; N]
    where
        T: FromNormalized + Default + Copy;
}

impl Graph for [[f32; 2]] {
    fn sample_graph(&self, input: f32) -> f32 {
        let input = input.clamp(0.0, 1.0);

        match self.binary_search_by(|v| v[0].total_cmp(&input)) {
            Ok(index) => self[index][1],
            Err(after) => {
                let before = if after > 0 { self[after - 1] } else { [0.0; 2] };
                let after = if after < self.len() { self[after] } else { [1.0; 2] };
                let fraction = (input - before[0]) / (after[0] - before[0]);
                before[1] * (1.0 - fraction) + after[1] * fraction
            }
        }
    }

    #[allow(clippy::needless_range_loop)] // does it matters for performance here (bounds check)?
    fn make_1d_data<T, const N: usize>(&self) -> [T; N]
    where
        T: FromNormalized + Default + Copy,
    {
        let mut result = [T::default(); N];

        for i in 0..N {
            let normalized = self.sample_graph(i as f32 / (N - 1) as f32);
            result[i] = T::from_normalized(normalized);
        }

        result
    }
}

#[cfg(test)]
mod test {
    use crate::graph::Graph;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn empty_graph_sampling() {
        let graph: [[f32; 2]; _] = [];
        assert!((graph.sample_graph(0.0) - 0.0).abs() <= EPSILON);
        assert!((graph.sample_graph(1.0) - 1.0).abs() <= EPSILON);
    }

    #[test]
    fn single_point_graph_sampling() {
        let graph: [[f32; 2]; _] = [[0.5, 0.75]];
        assert!((graph.sample_graph(0.00) - 0.000).abs() <= EPSILON);
        assert!((graph.sample_graph(0.25) - 0.375).abs() <= EPSILON);
        assert!((graph.sample_graph(0.50) - 0.750).abs() <= EPSILON);
        assert!((graph.sample_graph(0.75) - 0.875).abs() <= EPSILON);
        assert!((graph.sample_graph(1.00) - 1.000).abs() <= EPSILON);
    }

    #[test]
    fn multiple_points_graph_sampling() {
        let graph: [[f32; 2]; _] = [[0.1, 0.65], [0.5, 0.75], [0.9, 0.85]];
        assert!((graph.sample_graph(0.00) - 0.00).abs() <= EPSILON);
        assert!((graph.sample_graph(0.30) - 0.70).abs() <= EPSILON);
        assert!((graph.sample_graph(0.50) - 0.75).abs() <= EPSILON);
        assert!((graph.sample_graph(0.70) - 0.80).abs() <= EPSILON);
        assert!((graph.sample_graph(1.00) - 1.00).abs() <= EPSILON);
    }

    #[test]
    fn empty_graph_array() {
        let graph: [[f32; 2]; _] = [];
        let data: [u8; 256] = graph.make_1d_data();

        for i in 0..256 {
            assert_eq!(data[i], i as u8);
        }
    }

    #[test]
    fn all_one_graph_array() {
        let graph: [[f32; 2]; _] = [[0.0, 1.0], [1.0, 1.0]];

        for i in 0..11 {
            let v = i as f32 / 10.0;
            assert!((graph.sample_graph(v) - 1.00).abs() <= EPSILON);
        }
    }
}
