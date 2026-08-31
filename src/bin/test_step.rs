use std::fs;
use il2cpp_dumper::il2cpp::metadata::Metadata;
use il2cpp_dumper::formats::elf::Elf;
use il2cpp_dumper::il2cpp::base::Il2Cpp;
use il2cpp_dumper::executor::Il2CppExecutor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let meta_path = if args.len() > 1 {
        args[1].clone()
    } else {
        r"E:\Game Automation\oxide_stuff\dump_v11895\global-metadata_decrypted.dat".to_string()
    };

    let so_path = if args.len() > 2 {
        args[2].clone()
    } else {
        r"E:\Game Automation\oxide_stuff\dump_v11895\libil2cpp.so".to_string()
    };

    let out_dir = if args.len() > 3 {
        args[3].clone()
    } else {
        r"E:\Game Automation\oxide_stuff\dump_v11895\final_dump".to_string()
    };

    println!("[1] Reading metadata from: {}", meta_path);
    let meta_bytes = fs::read(&meta_path)?;
    let mut metadata = Metadata::new_with_options(meta_bytes, None, false)?;
    println!("[1] Metadata loaded: {} types, {} methods", metadata.type_defs.len(), metadata.method_defs.len());

    println!("[2] Reading libil2cpp from: {}", so_path);
    let so_bytes = fs::read(&so_path)?;
    let is_64 = so_bytes.len() > 4 && so_bytes[4] == 2;
    let mut elf = Elf::new(so_bytes, !is_64)?;
    elf.set_properties(metadata.version, metadata.metadata_usages_count as u64);

    let method_count = metadata.method_defs.len();
    let type_count = metadata.type_defs.len();
    let image_count = metadata.image_defs.len();
    let mut helper = elf.get_section_helper(method_count, type_count, image_count);
    let code_reg = helper.find_code_registration().expect("CR not found");
    let metadata_reg = helper.find_metadata_registration().expect("MR not found");
    println!("[2] Found CR: 0x{:X}, MR: 0x{:X}", code_reg, metadata_reg);

    let mut il2cpp = Il2Cpp::from_elf(&elf);
    println!("[3] Initializing auto_plus...");
    il2cpp.init_with_auto_plus(code_reg, metadata_reg, &|addr| elf.map_vatr(addr), false)?;
    println!("[3] Il2Cpp types: {}, method_pointers: {}", il2cpp.types.len(), il2cpp.method_pointers.len());

    println!("[4] Creating Executor...");
    let mut executor = Il2CppExecutor::new(&metadata, &mut il2cpp)?;
    println!("[4] Executor ready.");

    println!("[5] Testing type name resolution for first 10 types...");
    for i in 0..10.min(metadata.type_defs.len()) {
        let td = &metadata.type_defs[i];
        let name = executor.get_type_def_name(td, i, &metadata, &il2cpp, true, true);
        println!("  Type {}: {}", i, name);
    }
    println!("[5] Type name resolution passed!");

    println!("[6] Running full decompiler to {}...", out_dir);
    std::fs::create_dir_all(&out_dir)?;
    let mut config = il2cpp_dumper::config::Config::default();
    config.dump_disassembly = false;
    config.generate_struct = false;
    config.generate_dummy_dll = false;

    il2cpp_dumper::output::decompiler::Il2CppDecompiler::decompile(
        &mut executor,
        &metadata,
        &il2cpp,
        &config,
        &out_dir,
        |msg| println!("  [DECOMPILER] {msg}"),
    )?;

    println!("[7] Generating script.json, stringliteral.json, and C/C++ headers...");
    il2cpp_dumper::output::struct_generator::StructGenerator::write_all(
        &mut executor,
        &mut metadata,
        &mut il2cpp,
        &config,
        &out_dir,
        None,
    )?;
    println!("[7] Full dump and headers generated SUCCEEDED!");

    Ok(())
}
