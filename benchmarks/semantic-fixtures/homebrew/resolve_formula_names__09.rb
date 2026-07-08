# ID: Library/Homebrew/utils/bottles.rb:70
def derive_formula_names(bottle_file)
  name = bottle_file_list(bottle_file).first.to_s.split("/").fetch(0)

  full_name = if (receipt_file_path = receipt_path(bottle_file))
    receipt_contents = file_from_bottle(bottle_file, receipt_file_path)
    tap = Tab.from_file_content(receipt_contents, "#{bottle_file}/#{receipt_file_path}").tap
    "#{tap}/#{name}" if tap.present? && !tap.core_tap?
  else
    json_path = Pathname(bottle_file.sub(/\.(\d+\.)?tar\.gz$/, ".json"))
    if json_path.exist? && (raw = json_path.read.presence) &&
       (parsed = JSON.parse(raw).presence) && parsed.is_a?(Hash)
      parsed.keys.first.presence
    end
  end

  [name, full_name || name]
end
