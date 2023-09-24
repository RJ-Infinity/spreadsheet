use std::fs;
use std::process;
use std::env;
use std::ops::Index;
use std::boxed::Box;

fn exit_with_err_at(file_path: &str, line_i: usize, col_i: usize, msg: &str) -> !{
	eprintln!("{}:{}:{} Error: {}", file_path, line_i+1, col_i+1, msg);
	process::exit(1);
}

#[derive(Clone, Debug)]
pub struct Coord {
	x: usize,
	y: usize,
}

#[derive(Clone, Debug)]
pub enum Formula {
	Function(String, Vec<Box<Formula>>),
	Reference(Coord),
	Litteral(Box<CellValue>),
}

#[derive(Clone, Debug)]
pub enum CellErr {
	InvalidType,
}

#[derive(Clone, Debug)]
pub enum CellValue {
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
	file_path: String,
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
	pub fn new(file_path:String, str: String) -> Self {
		return Sheet { cells: str.replace("\r","").split("\n").enumerate().map(|(i, line)| {
			let mut rv: Vec<Cell> = Vec::new();
			let mut curr: Option<CellValue> = None;
			let mut tmp_str = String::new();
			let mut val_end = false;
			let mut in_esc = false;
			for (j, chr) in line.chars().enumerate() {
				// print!("===========================\nchr: {:#?}\nrv: {:#?}\ncurr: {:#?}\ntmp_str: {:#?}\nval_end: {:#?}\nin_esc: {:#?}\n",chr,rv,curr,tmp_str,val_end,in_esc);
				tmp_str.push(chr);
				match curr {
					None => {
						if chr == ',' {
							Self::append_cell(&mut rv, &mut tmp_str, CellValue::Empty, &mut curr);
						}else{
							curr = match chr {
								' ' | '\t' => None,
								'"' => Some(CellValue::String(String::new())),
								'=' => Some(CellValue::Formula(Formula::Litteral(CellValue::Empty.into()))),
								'-' | '+' | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => Some(CellValue::Number(0.0)),
								c => exit_with_err_at(&file_path, i, j, format!("could not parse `{}` as any valid type", c).as_str())
							}
						}
					}
					Some(ref mut v) => match v {
						CellValue::String(ref mut s) => {
							if val_end { 
								if chr == ',' {
									Self::append_cell(&mut rv, &mut tmp_str, v.clone(), &mut curr);
									val_end = false;
								}
							} else if in_esc {
								s.push(match chr {
									'\\' | '"' => chr,
									'n' => '\n',
									't' => '\t',
									'r' => '\r',
									_ => exit_with_err_at(&file_path, i, j-1, "Unknown escape sequence."), //remove 1 from j to get the slash loc
								});
								in_esc = false;
							} else { match chr {
								'\\' => {in_esc = true;},
								'"' => {val_end = true},
								c => {s.push(c)},
							}}
						}
						CellValue::Number(ref mut n) => {if chr == ',' {
							*n = tmp_str.strip_suffix(",").unwrap().trim().replace("_", "").parse().unwrap();
							Self::append_cell(&mut rv, &mut tmp_str, v.clone(), &mut curr);
							val_end = false;
						} else if !val_end { match chr {
							'0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '_' => {/* do nothing */},
							' ' | '\t' => val_end = true,
							'-' | '+' => exit_with_err_at(&file_path, i, j, "Plus and Minus symbols can only be at the start of a number. To calculate a value use a formula."),
							d => exit_with_err_at(&file_path, i, j, format!("`{}` is not a valid digit in what was parsed as a number if you want a string use quotes (`\"`) if you want an formula start it with an equals (`=`).", d).as_str()),
						}}else if chr != ' ' && chr != '\t' { exit_with_err_at(&file_path, i, j, "Only whitespace allowed after the end of a number. If you want a break in the number use an underscore (`_`).")}},
						CellValue::Formula(_f) => todo!(),
						_ => unreachable!(),
					}
				}
			}
			if let Some(ref mut v) = curr {
				match v {
					CellValue::String(_) => {},
					CellValue::Number(ref mut n) => { *n = tmp_str.trim().replace("_", "").parse().unwrap(); }
					CellValue::Formula(_f) => todo!(),
					_ => unreachable!(),
				}
				rv.push(Cell {
					value: v.clone(),
					string_backing: Some(tmp_str),
					changed: false,
				});
			}
			return rv;
		}).collect::<Vec<_>>(), file_path: file_path };
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
		eprintln!("Error no input file");
		process::exit(0);
	}else if args.len() > 2 {
		eprintln!("Error to many arguments");
		process::exit(0);
	}

	let sheet = Sheet::new(args[1].clone(), fs::read_to_string(&args[1]).unwrap_or_else(|err|{
		eprintln!("Error could not read file due to `{}`", err);
		process::exit(0);
	}));
	println!("{:#?}", sheet);
}
