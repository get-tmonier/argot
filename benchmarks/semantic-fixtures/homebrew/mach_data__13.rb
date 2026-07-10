# ID: Library/Homebrew/os/mac/mach.rb:30
def parse_mach_data(binary)
  macho = binary.macho
  machos = macho.is_a?(MachO::FatFile) ? macho.machos : [macho]

  machos.map do |m|
    arch = case m.cputype
    when :x86_64, :i386, :ppc64, :arm64, :arm then m.cputype
    when :ppc then :ppc7400
    else :dunno
    end

    type = case m.filetype
    when :dylib, :bundle then m.filetype
    when :execute then :executable
    else :dunno
    end

    { arch:, type: }
  end
rescue MachO::NotAMachOError
  []
rescue
  onoe "Failed to read Mach-O binary: #{binary}"
  raise if Homebrew::EnvConfig.developer?

  []
end
