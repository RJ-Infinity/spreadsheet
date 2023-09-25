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
	changed: bool,
}
#[derive(Debug)]
pub struct Sheet{
	cells: Vec<Vec<Cell>>,
	file_path: String,
}
impl Sheet {
	fn get_next_value(file_path: &str, line_no: usize, line: &str, start: usize, mut curr: Option<CellValue>) -> (Option<Cell>, Option<char>){
		let mut i = start;
		let mut val_end = false;
		let mut in_esc = false;
		let mut ws_count = 0;
		let mut fn_start = 0;
		while i < line.len() {
			let chr = line.chars().nth(i).unwrap();
			// print!("===========================\nchr: {:#?}\ni: {:#?}\nval_end: {:#?}\nin_esc: {:#?}\nws_count: {:#?}\nfn_start: {:#?}\n",chr,i,val_end,in_esc,ws_count,fn_start);
			match curr {
				None => {
					if chr == ',' || chr == ')' {
						return (Some(Cell {
							value: CellValue::Empty,
							string_backing: Some(line[start..i].to_string()),
							changed: false,
						}), Some(chr));
					}else{curr = match chr {
						' ' | '\t' => None,
						'"' => Some(CellValue::String(String::new())),
						'=' => Some(CellValue::Formula(Formula::Litteral(CellValue::Empty.into()))),
						'-' | '+' | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => Some(CellValue::Number(0.0)),
						c => exit_with_err_at(&file_path, line_no, i, format!("could not parse `{}` as any valid type", c).as_str())
					}}
				}
				Some(ref mut v) => match v {
					CellValue::String(ref mut s) => {
						if val_end {
							if chr == ',' || chr == ')' { return (Some(Cell {
								value: v.clone(),
								string_backing: Some(line[start..i].to_string()),
								changed: false,
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
					}
					CellValue::Number(ref mut n) => {if chr == ',' || chr == ')' {
						let num_str = line[start..i].trim().replace("_", "");
						if num_str == "+" || num_str == "-" { exit_with_err_at(&file_path, line_no, i, "A number must contain more than just the sign."); }
						*n = num_str.parse().unwrap();
						return (Some(Cell {
							value: v.clone(),
							string_backing: Some(line[start..i].to_string()),
							changed: false,
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
							changed: false,
						}), Some(chr));}else if chr != ' ' && chr != '\t' { exit_with_err_at(&file_path, line_no, i, "Only whitespace alowed after the end of a function call."); }
					}else{
						if chr >= 'a' && chr <= 'z' {
							if fn_start == 0 {fn_start = i;}
						}else if fn_start > 0 && chr == '('{
							*formula = Formula::Function(
								line[fn_start..i].to_string(),
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

									// println!("{:#?}",cell.clone().unwrap().string_backing);

									i += cell.unwrap().string_backing.unwrap().len() + 1;
									
									if end_chr == Some(')'){
										println!("{:#?}", line.chars().nth(i));
										val_end = true;
										break;
									} else if end_chr != Some(',')
									{exit_with_err_at(&file_path, line_no, i, "Must close the bracket previously opened.")}
								}
							}
						}else if (chr >= '0' && chr <= '9') || chr == '+' || chr == '-' || chr == '"'{
							// println!("RECURSE");
							let value = Self::get_next_value(file_path, line_no, line, i-ws_count, None);
							// println!("END RECURSE");
							*formula = Formula::Litteral(value.clone().0.unwrap().value.into());
							// println!("UNDER VAL {:#?}",value.0.clone().unwrap().string_backing.unwrap());
							i += value.0.unwrap().string_backing.unwrap().len()-ws_count;
							return (Some(Cell {
								value: v.clone(),
								string_backing: Some(line[start..i].to_string()),
								changed: false,
							}), value.1);
						}else if chr == ' ' || chr == '\t' { ws_count += 1; }
						else {exit_with_err_at(&file_path, line_no, i, format!("`{}` is not valid character in this part of a formula", chr).as_str())}
					}},
					_ => unreachable!(),
				}
			}
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
				CellValue::Formula(_f) => {},
				_ => unreachable!(),
			}
			return (Some(Cell {
				value: v.clone(),
				string_backing: Some(line[start..].to_string()),
				changed: false,
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
