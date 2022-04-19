use super::Volume;

pub struct VolumeIterator<
    'a, 
    T: Sized, 
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> {
    pub volume: &'a Volume<T, X_SIZE, Y_SIZE, Z_SIZE>, 
    idx: [usize; 3]
}

pub struct VolumeIdxIterator< 
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> {
    idx: [usize; 3]
}

impl<
    'a, 
    T: Sized, 
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> VolumeIterator<'a, T, X_SIZE, Y_SIZE, Z_SIZE> {
    pub fn new(volume: &'a Volume<T, X_SIZE, Y_SIZE, Z_SIZE>) -> Self {
        Self {
            volume,
            idx: [0, 0, 0]
        }
    }
}

impl<
    'a, 
    T: Sized, 
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> Iterator for VolumeIterator<'a, T, X_SIZE, Y_SIZE, Z_SIZE> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // Extract the element at the current index.
        let out = if self.idx[2] >= Z_SIZE {
            None
        } else {
            Some(&self.volume[self.idx])
        };
            
        // Increment the 'index vector' here. 
        // Code looks a bit ugly but it's the most clear and readable implementation I could come up with.
        self.idx[0] += 1;
        
        if self.idx[0] >= X_SIZE {

            self.idx[0] = 0;
            self.idx[1] += 1;
            
            if self.idx[1] >= Y_SIZE {
                
                self.idx[1] = 0;
                self.idx[2] += 1;
            }
        }

        out
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let iterator_size = X_SIZE * Y_SIZE * Z_SIZE;
        (iterator_size, Some(iterator_size))
    }
}

impl<
    'a, 
    T: Sized, 
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> ExactSizeIterator for VolumeIterator<'a, T, X_SIZE, Y_SIZE, Z_SIZE> {
    // Default implementations do everything we need here.
}

impl< 
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> Default for VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
    fn default() -> Self {
        Self {
            idx: [0, 0, 0]
        }
    }
}

impl<
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> Iterator for VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
    type Item = na::Vector3<usize>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        
        // Extract the element at the current index.
        let out = if self.idx[2] >= Z_SIZE {
            None
        } else {
            Some(self.idx.into())
        };
        
        // Increment the 'index vector' here. 
        // Code looks a bit ugly but it's the most clear and readable implementation I could come up with.
        self.idx[0] += 1;
        
        if self.idx[0] >= X_SIZE {

            self.idx[0] = 0;
            self.idx[1] += 1;
            
            if self.idx[1] >= Y_SIZE {
                
                self.idx[1] = 0;
                self.idx[2] += 1;
            }
        }

        out
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let iterator_size = X_SIZE * Y_SIZE * Z_SIZE;
        (iterator_size, Some(iterator_size))
    }
}

impl<
    const X_SIZE: usize, 
    const Y_SIZE: usize, 
    const Z_SIZE: usize
> ExactSizeIterator for VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
    // Default implementations do everything we need here.
}