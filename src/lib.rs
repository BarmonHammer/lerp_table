use ordered_float::{FloatIsNan, NotNan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(try_from = "Vec<Coord>", into = "Vec<(NotNan<f64>, NotNan<f64>)>")]
pub struct Piecewise(Vec<Coord>);

#[derive(Error, Debug)]
pub enum PiecewiseErr {
    #[error("The provided segment is empty")]
    InputEmpty,
    #[error("The function is undefined")]
    InputUndefined,
    #[error("The value is not in the domain")]
    NotInDomain,
    #[error("The value provided is NaN")]
    InputNaN,
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Coord(NotNan<f64>, NotNan<f64>);

impl From<Coord> for (NotNan<f64>, NotNan<f64>) {
    fn from(value: Coord) -> Self {
        (value.0, value.1)
    }
}

impl<X: Into<f64>, Y: Into<f64>> TryFrom<(X, Y)> for Coord {
    type Error = FloatIsNan;
    fn try_from(value: (X, Y)) -> Result<Self, Self::Error> {
        Ok(Coord(
            NotNan::new(value.0.into())?,
            NotNan::new(value.1.into())?,
        ))
    }
}

impl Coord {
    pub const unsafe fn new_unchecked(value: (f64, f64)) -> Self {
        Self(
            NotNan::new_unchecked(value.0),
            NotNan::new_unchecked(value.1),
        )
    }
    pub const fn zero() -> Self {
        unsafe { Self(NotNan::new_unchecked(0.0), NotNan::new_unchecked(0.0)) }
    }
}
//takes a bit to load, but verification is verification
impl TryFrom<Vec<Coord>> for Piecewise {
    type Error = PiecewiseErr;
    fn try_from(mut points: Vec<Coord>) -> Result<Self, Self::Error> {
        match points.len() {
            0 => return Err(PiecewiseErr::InputEmpty),
            1 => return Ok(Piecewise(points.into())),
            _ => (),
        }

        points.sort_by(|a, b| a.0.cmp(&b.0));

        for point_pair in points.windows(2) {
            let Coord(x1, y1) = point_pair[0];
            let Coord(x2, y2) = point_pair[1];

            if x2 == x1 && y2 != y1 {
                return Err(PiecewiseErr::InputUndefined);
            }
        }

        Ok(Piecewise(points.into()))
    }
}

impl From<Piecewise> for Vec<(NotNan<f64>, NotNan<f64>)> {
    fn from(value: Piecewise) -> Self {
        let mut buffer = Vec::new();
        for x in value.as_slice() {
            buffer.push((*x).into());
        }
        buffer
    }
}

impl Piecewise {
    fn as_slice(&self) -> &[Coord] {
        self.0.as_slice()
    }
    pub fn y_at_x(&self, value: f64) -> Result<f64, PiecewiseErr> {
        let value = NotNan::new(value).map_err(|_| PiecewiseErr::InputNaN)?;
        let data = self.as_slice();
        //since we know the domains have to be sorted (try_from will result Err if not)
        //we can binary search the domains to find the domain needed
        let bsearch = data.binary_search_by(|point| point.0.cmp(&value));

        let index = match bsearch {
            //checks to see if the value is out of out domains bound
            Err(x) if x == 0 || x - 1 > data.len() => return Err(PiecewiseErr::NotInDomain),
            //if not out of bounds then x is the index of the next point
            //ie. (0,0), (100, 0) and we supply 50 x will be the index of (100, 0)
            Err(x) => x,
            //if bsearch returns Ok(x) it means we landed on an exact point,
            //so we can return that value without doing any math
            Ok(x) => return Ok(data[x].1.into_inner()),
        };

        let Coord(x1, y1) = data[index - 1];
        let Coord(x2, y2) = data[index];

        let slope = (y1 - y2) / (x1 - x2);

        Ok((slope * (value - x1) + y1).into_inner())
    }
}

#[cfg(test)]
mod tests {

    use crate::Coord;
    use crate::Piecewise;
    const SIDEARM: [Coord; 3] = unsafe {
        [
            Coord::new_unchecked((0.0, 18.0)),
            Coord::new_unchecked((90.0, 36.0)),
            Coord::new_unchecked((100.0, 42.0)),
        ]
    };

    #[test]
    fn try_from() {
        let vec: Vec<Coord> = Vec::from(SIDEARM);
        let z: Piecewise = Piecewise::try_from(vec).unwrap();
        //let z: U8PieceWise = U8PieceWise::try_from((&x, &y)).unwrap();
        assert_eq!(z.y_at_x(33.0.try_into().unwrap()).unwrap().floor(), 24.0);
        assert_eq!(z.y_at_x(93.0).unwrap().floor(), 37.0);
    }

    #[test]
    fn serialize() {
        let vec: Vec<Coord> = Vec::from(SIDEARM);
        let z: Piecewise = vec.try_into().unwrap();
        assert_eq!(
            serde_json::to_string(&z).unwrap(),
            "[[0.0,18.0],[90.0,36.0],[100.0,42.0]]"
        );
    }
    #[test]
    fn deserialize() {
        let z: Piecewise = serde_json::from_str("[[0,18],[90,36.0],[100.0,42.0]]").unwrap();
        assert_eq!(
            serde_json::to_string(&z).unwrap(),
            "[[0.0,18.0],[90.0,36.0],[100.0,42.0]]"
        );
    }
}
