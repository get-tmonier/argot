# ID: Library/Homebrew/utils/inreplace.rb:55
def replace_in_files(paths, before = nil, after = nil, audit_result: true, global: true, &block)
  paths = Array(paths)
  after &&= after.to_s
  before = before.to_s if before.is_a?(Pathname)

  errors = {}
  errors["`paths` (first) parameter"] = ["`paths` was empty"] if paths.all?(&:blank?)

  paths.each do |path|
    contents = File.binread(path)
    extension = StringInreplaceExtension.new(contents)

    if before.nil? && after.nil?
      raise ArgumentError, "Must supply a block or before/after params" unless block

      yield extension
    elsif global
      extension.gsub!(before, after, audit_result:)
    else
      extension.sub!(before, after, audit_result:)
    end

    errors[path] = extension.errors unless extension.errors.empty?
    Pathname(path).atomic_write(extension.inreplace_string)
  end

  raise Utils::Inreplace::Error, errors if errors.present?
end
