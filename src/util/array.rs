use crate::util::s_array;

pub struct Array1<T> {
    array: s_array::SArray<T>,
}

impl<T: Copy> Array1<T> {
    pub fn new(size: u32) -> Array1<T> {
        Array1 {
            array: s_array::SArray::new(1, vec![size]),
        }
    }

    pub fn len(&self) -> u32 {
        self.array.get_len()
    }

    pub fn init(&mut self, value: T) {
        self.array.init_array(value);
    }

    pub fn get(&self, index: u32) -> T {
        self.array.get(vec![index])
    }

    pub fn set(&mut self, index: u32, value: T) {
        self.array.set(vec![index], value);
    }

    pub fn iter(&self) -> Iter1<'_, T> {
        Iter1::new(&self.array)
    }
}

pub struct Iter1<'a, T> {
    array: &'a s_array::SArray<T>,
    index: u32,
    size: u32,
}

impl<'a, T: Copy> Iter1<'a, T> {
    fn new(array: &'a s_array::SArray<T>) -> Iter1<'a, T> {
        Iter1 {
            array,
            index: 0,
            size: array.get_size()[0],
        }
    }
}

impl<'a, T: Copy> Iterator for Iter1<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.array.get_len() {
            let index = self.index;
            self.index += 1;
            return Some(self.array.get(vec![index]));
        }
        None
    }
}

pub struct Array2<T> {
    array: s_array::SArray<T>,
}

impl<T: Copy> Array2<T> {
    pub fn new(row: u32, column: u32) -> Array2<T> {
        Array2 {
            array: s_array::SArray::new(2, vec![row, column]),
        }
    }

    pub fn len(&self) -> u32 {
        self.array.get_len()
    }

    pub fn size(&self) -> [u32; 2] {
        let size = self.array.get_size();
        [size[0], size[1]]
    }

    pub fn init(&mut self, value: T) {
        self.array.init_array(value);
    }

    pub fn get(&self, row: u32, column: u32) -> T {
        self.array.get(vec![row, column])
    }

    pub fn set(&mut self, row: u32, column: u32, value: T) {
        self.array.set(vec![row, column], value);
    }

    pub fn iter(&self) -> Iter2<'_, T> {
        Iter2::new(&self.array)
    }
}

pub struct Iter2<'a, T> {
    array: &'a s_array::SArray<T>,
    index: u32,
    size: [u32; 2],
}

impl<'a, T: Copy> Iter2<'a, T> {
    fn new(array: &'a s_array::SArray<T>) -> Iter2<'a, T> {
        Iter2 {
            array,
            index: 0,
            size: [array.get_size()[0], array.get_size()[1]],
        }
    }
}

impl<'a, T: Copy> Iterator for Iter2<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.array.get_len() {
            let i = self.index / self.size[1];
            let j = self.index % self.size[1];
            self.index += 1;
            return Some(self.array.get(vec![i, j]));
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basics() {
        let mut arr_1 = Array1::<u8>::new(10);
        arr_1.init(1);
        arr_1.set(8, 2);
        assert_eq!(arr_1.get(8), 2);
        assert_eq!(arr_1.get(0), 1);
        assert_eq!(arr_1.get(9), 1);
        assert_eq!(arr_1.len(), 10);

        let mut arr_1 = Array1::<u32>::new(12);
        arr_1.init(11);
        arr_1.set(0, 3);
        arr_1.set(7, 4);
        arr_1.set(11, 5);
        let data: [u32; 12] = [3, 11, 11, 11,
                               11, 11, 11, 4,
                               11, 11, 11, 5];
        for (i, temp) in arr_1.iter().enumerate() {
            assert_eq!(temp, data[i]);
        }

        let mut arr_2 = Array2::<u8>::new(10000, 10000);
        arr_2.init(100);
        arr_2.set(8000, 6752, 67);
        assert_eq!(arr_2.get(8000, 6752), 67);
        assert_eq!(arr_2.get(0, 0), 100);
        assert_eq!(arr_2.get(9999, 9999), 100);
        assert_eq!(arr_2.len(), 10000 * 10000);
        assert_eq!(arr_2.size(), [10000, 10000]);

        let mut arr_2 = Array2::<u32>::new(3, 4);
        arr_2.init(11);
        arr_2.set(0, 3, 67);
        arr_2.set(1, 3, 67);
        arr_2.set(2, 3, 67);
        let data: [u32; 12] = [11, 11, 11, 67,
                               11, 11, 11, 67,
                               11, 11, 11, 67];
        for (i, temp) in arr_2.iter().enumerate() {
            assert_eq!(temp, data[i]);
        }
    }
}