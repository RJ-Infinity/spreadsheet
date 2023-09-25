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
#[derive(Clone, Debug)]
pub struct Cell{
	value: CellValue,
	string_backing: Option<String>,
}
#[derive(Debug)]
pub struct Sheet{
	cells: Vec<Vec<Cell>>,
	file_path: String,
}
#[derive(Debug)]
pub enum CoordFromRefErr{
	InvalidLetter,
	InvalidNumber,
}
impl Sheet {
	pub fn coord_from_ref(refer: &str, split: Option<(usize, usize, usize)>) -> Result<Coord,CoordFromRefErr> {
		if split == None{ todo!(); }
		let split = split.unwrap();
		let mut j = split.1 - 1;
		let mut mult = 26;
		let mut x = 0;

		let mut chr = refer.chars().nth(j).unwrap();
		if chr < 'A' || chr > 'Z' {return Err(CoordFromRefErr::InvalidLetter); }
		x += chr as usize - 'A' as usize;
		j-=1;
		while j >= split.0 {
			chr = refer.chars().nth(j).unwrap();
			if chr < 'A' || chr > 'Z' {return Err(CoordFromRefErr::InvalidLetter); }
			x += (chr as usize - 'A' as usize + 1)*mult;
			mult*=26;
			j-=1;
		}
		return Ok(Coord { x: x, y: match refer[split.1..split.2].parse(){
			Ok(y) => y,
			Err(_) => return Err(CoordFromRefErr::InvalidNumber),
		}});
	}
	fn get_next_value(file_path: &str, line_no: usize, line: &str, start: usize, mut curr: Option<CellValue>) -> (Option<Cell>, Option<char>){
		let mut i = start;
		let mut val_end = false;
		let mut in_esc = false;
		let mut ws_count = 0;
		let mut fn_start = None;
		let mut ref_start = (None,None);
		while i < line.len() {
			let chr = line.chars().nth(i).unwrap();
			// print!("===========================\nchr: {:#?}\ni: {:#?}\nval_end: {:#?}\nin_esc: {:#?}\nws_count: {:#?}\nfn_start: {:#?}\n",chr,i,val_end,in_esc,ws_count,fn_start);
			match curr {None => {
				if chr == ',' || chr == ')' {
					return (Some(Cell {
						value: CellValue::Empty,
						string_backing: Some(line[start..i].to_string()),
					}), Some(chr));
				}else{curr = match chr {
					' ' | '\t' => None,
					'"' => Some(CellValue::String(String::new())),
					'=' => Some(CellValue::Formula(Formula::Litteral(CellValue::Empty.into()))),
					'-' | '+' | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => Some(CellValue::Number(0.0)),
					c => exit_with_err_at(&file_path, line_no, i, format!("could not parse `{}` as any valid type", c).as_str())
				}}
			},Some(ref mut v) => match v {
				CellValue::String(ref mut s) => {
					if val_end {
						if chr == ',' || chr == ')' { return (Some(Cell {
							value: v.clone(),
							string_backing: Some(line[start..i].to_string()),
						}), Some(chr));}else if chr != ' ' && chr != '\t' { exit_with_err_at(&file_path, line_no, i, "Only whitespace alowed after the end of a string."); }
					} else if in_esc {
						s.push(match chr {
							'\\' | '"' => chr,
							'n' => '\n',
							't' => '\t',
							'r' => '\r',
							_ => exit_with_err_at(&file_path, line_no, i-1, "Unknown escape sequence."), //remove 1 from j to get the slash loc
						});
						in_esc = false;
					} else { match chr {
						'\\' => {in_esc = true;},
						'"' => {val_end = true},
						c => {s.push(c)},
					}}
				},
				CellValue::Number(ref mut n) => {if chr == ',' || chr == ')' {
					let num_str = line[start..i].trim().replace("_", "");
					if num_str == "+" || num_str == "-" { exit_with_err_at(&file_path, line_no, i, "A number must contain more than just the sign."); }
					*n = num_str.parse().unwrap();
					return (Some(Cell {
						value: v.clone(),
						string_backing: Some(line[start..i].to_string()),
					}), Some(chr));
				} else if !val_end { match chr {
					'0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '_' => {/* do nothing */},
					' ' | '\t' => val_end = true,
					'-' | '+' => exit_with_err_at(&file_path, line_no, i, "Plus and Minus symbols can only be at the start of a number. To calculate a value use a formula."),
					d => exit_with_err_at(&file_path, line_no, i, format!("`{}` is not a valid digit in what was parsed as a number if you want a string use quotes (`\"`) if you want an formula start it with an equals (`=`).", d).as_str()),
				}}else if chr != ' ' && chr != '\t' { exit_with_err_at(&file_path, line_no, i, "Only whitespace allowed after the end of a number. If you want a break in the number use an underscore (`_`).")}},
				CellValue::Formula(ref mut formula) => {if val_end {
					if chr == ',' || chr == ')' { return (Some(Cell {
						value: v.clone(),
						string_backing: Some(line[start..i].to_string()),
					}), Some(chr));}else if chr != ' ' && chr != '\t' { exit_with_err_at(&file_path, line_no, i, "Only whitespace alowed after the end of a function call or reference."); }
				}else{
					if chr >= 'A' && chr <= 'Z' {
						if fn_start.is_some() {exit_with_err_at(&file_path, line_no, i, "characters in a function must all be lowercase");}
						if ref_start.0 == None {ref_start.0 = Some(i);}
					}else if chr >= 'a' && chr <= 'z' {
						if ref_start.0.is_some() {exit_with_err_at(&file_path, line_no, i, "characters in a reference must all be uppercase");}
						if fn_start == None {fn_start = Some(i);}
					}else if fn_start.is_some() && chr == '('{
						*formula = Formula::Function(
							line[fn_start.unwrap()..i].to_string(),
							Vec::new()
						);
						while {
							let next_chr = line.chars().nth(i+1);
							next_chr == Some(' ') || next_chr == Some('\t')
						} { i+=1; }
						if line.chars().nth(i+1) == Some(')') {
							i+=1;
							val_end = true;
						}else{
							let mut cell;
							let mut end_chr;
							loop {
								(cell, end_chr) = Self::get_next_value(file_path,line_no,line,i+1,Some(
									CellValue::Formula(Formula::Litteral(CellValue::Empty.into()))
								));
								let value = cell.clone().unwrap_or_else(
									|| exit_with_err_at(&file_path, line_no, i, "Must close the bracket previously opened.")
								);

								match value.value { CellValue::Formula(f) => {
									match formula { Formula::Function(_, args) =>{
										args.push(f.into());
									},_ => unreachable!()}
								}, _ => unreachable!(),}

								i += cell.unwrap().string_backing.unwrap().len() + 1;
								
								if end_chr == Some(')'){
									val_end = true;
									break;
								} else if end_chr != Some(',')
								{exit_with_err_at(&file_path, line_no, i, "Must close the bracket previously opened.")}
							}
						}
					}else if ref_start.0.is_some() && chr >= '0' && chr <= '9'{
						if ref_start.1 == None {ref_start.1 = Some(i);}
					}else if ref_start.0 == None && ((chr >= '0' && chr <= '9') || chr == '+' || chr == '-' || chr == '"') {
						let value = Self::get_next_value(file_path, line_no, line, i-ws_count, None);
						*formula = Formula::Litteral(value.clone().0.unwrap().value.into());
						i += value.0.unwrap().string_backing.unwrap().len()-ws_count;
						return (Some(Cell {
							value: v.clone(),
							string_backing: Some(line[start..i].to_string()),
						}), value.1);
					}else if chr == ' ' || chr == '\t' {
						if fn_start.is_some() { exit_with_err_at(&file_path, line_no, i, "Expected a bracket (`(`) after the name of a function not a whitespace character."); }
						if ref_start.0.is_some() {
							if ref_start.1 == None {exit_with_err_at(&file_path, line_no, i, "A reference requires a numeric component however got a whitespace character.")}
							*formula = Formula::Reference(Self::coord_from_ref(
								line,
								Some((ref_start.0.unwrap(),ref_start.1.unwrap(),i))
							).unwrap());
							val_end = true;
						}
						ws_count += 1;
					}
					else {exit_with_err_at(&file_path, line_no, i, format!("`{}` is not valid character in this part of a formula", chr).as_str())}
				}}, _ => unreachable!(),
			}}
			i+=1;
		}
		if let Some(ref mut v) = curr {
			match v {
				CellValue::String(_) => if !val_end {exit_with_err_at(&file_path, line_no, i, "There is no closing quote.");},
				CellValue::Number(ref mut n) => {
					let num_str = line[start..i].trim().replace("_", "");
					if num_str == "+" || num_str == "-" { exit_with_err_at(&file_path, line_no, i, "A number must contain more than just the sign."); }
					*n = num_str.parse().unwrap();
				}
				CellValue::Formula(f) => if ref_start.0.is_some() && !matches!(f, Formula::Reference(_)) {
					if ref_start.1 == None {exit_with_err_at(&file_path, line_no, i, "A reference requires a numeric component however got a whitespace character.")}
					*f = Formula::Reference(Self::coord_from_ref(
						line,
						Some((ref_start.0.unwrap(),ref_start.1.unwrap(),i))
					).unwrap());
				},
				_ => unreachable!(),
			}
			return (Some(Cell {
				value: v.clone(),
				string_backing: Some(line[start..].to_string()),
			}), None);
		}
		return (None, None);
	}
	pub fn new(file_path:String, str: String) -> Self {
		return Sheet { cells: str.replace("\r","").split("\n").enumerate().map(|(i, line)| {
			let mut rv: Vec<Cell> = Vec::new();
			let mut start = 0;
			let mut cell;
			let mut end_chr;
			let mut chr_i = 0;
			(cell, end_chr) = Self::get_next_value(&file_path, i, line, start, None);
			while let Some(ref c) = cell {
				chr_i += c.string_backing.as_ref().unwrap().len()+1;
				if end_chr != None && end_chr != Some(',') {
					exit_with_err_at(&file_path, i, chr_i-1, "invalid character");
				}
				rv.push((*c).clone());
				start += c.string_backing.as_ref().unwrap().len() + 1;
				(cell, end_chr) = Self::get_next_value(&file_path, i, line, start, None);
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
