use core::panic;
use std::borrow::BorrowMut;
use std::fs;
use std::process;
use std::env;
use std::ops::Index;

pub struct Coord {
	x: usize,
	y: usize,
}

#[derive(Clone, Debug)]
pub struct Formula {

}

#[derive(Clone, Debug)]
enum CellErr {
	InvalidType,
}

#[derive(Clone, Debug)]
enum CellValue {
	Empty,
	String(String),
	Number(f64),
	Formula(Formula),
	Err(CellErr),
}
#[derive(Debug)]
pub struct Cell{
	value: CellValue,
	string_backing: Option<String>,
	changed: bool,
}
#[derive(Debug)]
pub struct Sheet{
	cells: Vec<Vec<Cell>>,
}
impl Sheet {
	fn append_cell(cells: &mut Vec<Cell>, tmp_str: &mut String, value: CellValue, curr: &mut Option<CellValue>) {
		tmp_str.pop();
		cells.push(Cell {
			value: value,
			string_backing: Some(tmp_str.clone()),
			changed: false,
		});
		tmp_str.clear();
		*curr = None;
	}
	pub fn new(str: String) -> Self {
		return Sheet { cells: str.split("\n").map(|line| {
			let mut rv: Vec<Cell> = Vec::new();
			let mut curr: Option<CellValue> = None;
			let mut tmp_str = String::new();
			let mut str_end = false;
			let mut in_esc = false;
			for chr in line.chars() {
				// print!("===========================\nchr: {:#?}\nrv: {:#?}\ncurr: {:#?}\ntmp_str: {:#?}\nstr_end: {:#?}\nin_esc: {:#?}\n",chr,rv,curr,tmp_str,str_end,in_esc);
				tmp_str.push(chr);
				match curr {
					None => {
						if chr == ',' {
							Self::append_cell(&mut rv, &mut tmp_str, CellValue::Empty, &mut curr);
						}else{
							curr = match chr {
								' ' | '\t' => None,
								'"' => Some(CellValue::String(String::new())),
								'=' => Some(CellValue::Formula(Formula {})),
								_ => Some(CellValue::Number(0.0)),
							}
						}
					}
					Some(ref mut v) => match v {
						CellValue::String(ref mut s) => {
							if str_end { 
								if chr == ',' {
									Self::append_cell(&mut rv, &mut tmp_str, v.clone(), &mut curr);
									str_end = false;
								}
							} else if in_esc { s.push(match chr {
								'\\' | '"' => chr,
								'n' => '\n',
								't' => '\t',
								'r' => '\r',
								_ => panic!("Unknown escape sequence"),
							});} else { match chr {
								'\\' => {in_esc = true;},
								'"' => {str_end = true},
								c => {s.push(c)},
							}}
						}
						CellValue::Number(_n) => todo!(),
						CellValue::Formula(_f) => todo!(),
						_ => unreachable!(),
					}
				}
			}
			return rv;
		}).collect::<Vec<_>>() };
	}
}
impl Index<Coord> for Sheet {
	type Output = Cell;
	fn index(&self, c: Coord) -> &Cell {
		match self.cells.get(c.x).and_then(|cell|cell.get(c.y)) {
			Some(cell) => &cell,
			None => &Cell {
				value: CellValue::Empty,
				string_backing: None,
				changed: true,
			}
		}
	}
}


fn main() {
	let args: Vec<String> = env::args().collect();

	if args.len() == 1 {
		println!("Error no input file");
		process::exit(0);
	}else if args.len() > 2 {
		println!("Error to many arguments");
		process::exit(0);
	}

	let sheet = Sheet::new(match fs::read_to_string(&args[1]) {
		Ok(c) => c,
		Err(err) => {
			println!("Error could not read file due to `{}`", err);
			process::exit(0);
		}
	});
	print!("{:#?}", sheet);
}
