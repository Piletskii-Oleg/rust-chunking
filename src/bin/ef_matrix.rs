use rand::distributions::Distribution;
use rand::prelude::ThreadRng;
use rand::thread_rng;
use rand_distr::Normal;

const MATRIX_WIDTH: usize = 8;
const MATRIX_HEIGHT: usize = 255;

fn create_ef_matrix() -> Vec<Vec<u8>> {
    let base_matrix = (0..=255)
        .map(|index| vec![index; 5])
        .collect::<Vec<Vec<u8>>>(); // 256x5 matrix that looks like ((0,0,0,0,0), (1,1,1,1,1)..)

    let matrix_h = generate_matrix();
    let matrix_g = generate_matrix();

    let e_matrix = transform_base_matrix(&base_matrix, &matrix_h);
    let f_matrix = transform_base_matrix(&base_matrix, &matrix_g);

    let ef_matrix = e_matrix
        .iter()
        .zip(f_matrix.iter())
        .map(concatenate_bits_in_rows)
        .collect();
    ef_matrix
}

fn transform_base_matrix(
    base_matrix: &[Vec<u8>],
    additional_matrix: &[Vec<f64>],
) -> Vec<Vec<bool>> {
    base_matrix
        .iter()
        .map(|row| transform_byte_row(row[0], additional_matrix))
        .collect::<Vec<Vec<bool>>>()
}

fn concatenate_bits_in_rows((row_x, row_y): (&Vec<bool>, &Vec<bool>)) -> Vec<u8> {
    row_x
        .iter()
        .zip(row_y.iter())
        .map(concatenate_bits)
        .collect()
}

fn concatenate_bits((x, y): (&bool, &bool)) -> u8 {
    match (*x, *y) {
        (true, true) => 3,
        (true, false) => 2,
        (false, true) => 1,
        (false, false) => 0,
    }
}

fn transform_byte_row(byte: u8, matrix: &[Vec<f64>]) -> Vec<bool> {
    let mut new_row = [0u8; 5];
    (0..255)
        .map(|index| multiply_rows(byte, &matrix[index]))
        .enumerate()
        .for_each(|(index, value)| {
            if value > 0.0 {
                new_row[index / 51] += 1;
            }
        });

    new_row
        .iter()
        .map(|&number| number % 2 != 0)
        .collect::<Vec<bool>>()
}

fn multiply_rows(byte: u8, numbers: &[f64]) -> f64 {
    numbers
        .iter()
        .enumerate()
        .map(|(index, number)| {
            if (byte >> index) & 1 == 1 {
                *number
            } else {
                -(*number)
            }
        })
        .sum()
}

fn generate_matrix() -> Vec<Vec<f64>> {
    let normal = Normal::new(0.0, 1.0).unwrap();
    let mut rng = thread_rng();

    (0..MATRIX_HEIGHT)
        .map(|_| generate_row(&normal, &mut rng))
        .collect()
}

fn generate_row(normal: &Normal<f64>, rng: &mut ThreadRng) -> Vec<f64> {
    (0..MATRIX_WIDTH).map(|_| normal.sample(rng)).collect()
}

fn main() {
    let matrix = create_ef_matrix();
    println!("{:?}", matrix);
}
