use std::iter::Fuse;

pub struct Concat<I1, I2> {
	iter1: Fuse<I1>,
	iter2: Fuse<I2>,
}

impl<I1, I2> Concat<I1, I2>
where
	I1: Iterator,
	I2: Iterator<Item = I1::Item>,
{
	pub fn new(iter1: I1, iter2: I2) -> Self {
		Self {
			iter1: iter1.fuse(),
			iter2: iter2.fuse(),
		}
	}
}

impl<I1, I2> Iterator for Concat<I1, I2>
where
	I1: Iterator,
	I2: Iterator<Item = I1::Item>,
{
	type Item = I1::Item;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter1.next().or_else(|| self.iter2.next())
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let (min1, max1) = self.iter1.size_hint();
		let (min2, max2) = self.iter2.size_hint();

		(
			min1.saturating_add(min2),
			max1.and_then(|max1| max2.map(|max2| max1.saturating_add(max2))),
		)
	}
}
