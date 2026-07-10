# ID: python_modules/dagster/dagster/_config/type_printer.py:27
def _emit_config_type(config_schema_snapshot, config_type_key, printer, with_lines=True):
    break_line = printer.line if with_lines else lambda text: printer.append(text + " ")

    type_snap = config_schema_snapshot.get_config_snap(config_type_key)
    kind = type_snap.kind

    if kind == ConfigTypeKind.ARRAY:
        printer.append("[")
        _emit_config_type(config_schema_snapshot, type_snap.inner_type_key, printer)
        printer.append("]")
    elif kind == ConfigTypeKind.NONEABLE:
        _emit_config_type(config_schema_snapshot, type_snap.inner_type_key, printer)
        printer.append("?")
    elif kind == ConfigTypeKind.SCALAR_UNION:
        printer.append("(")
        _emit_config_type(config_schema_snapshot, type_snap.scalar_type_key, printer)
        printer.append(" | ")
        _emit_config_type(config_schema_snapshot, type_snap.non_scalar_type_key, printer)
        printer.append(")")
    elif kind == ConfigTypeKind.MAP:
        # e.g.
        # {
        #   [String]: Int
        # }
        break_line("{")
        with printer.with_indent():
            printer.append("[")
            # For a Map, given_name holds the optional key_label_name
            if type_snap.given_name:
                printer.append(f"{type_snap.given_name}: ")
            _emit_config_type(config_schema_snapshot, type_snap.key_type_key, printer)
            printer.append("]: ")
            _emit_config_type(
                config_schema_snapshot,
                type_snap.inner_type_key,
                printer,
                with_lines=with_lines,
            )
            break_line("")
        printer.append("}")
    elif ConfigTypeKind.has_fields(kind):
        break_line("{")
        with printer.with_indent():
            for field_snap in sorted(type_snap.fields):
                field_name = field_snap.name
                if field_snap.is_required:
                    printer.append(field_name + ": ")
                else:
                    printer.append(field_name + "?: ")
                _emit_config_type(
                    config_schema_snapshot,
                    field_snap.type_key,
                    printer,
                    with_lines=with_lines,
                )
                break_line("")

        printer.append("}")
    elif type_snap.given_name:
        printer.append(type_snap.given_name)
    else:
        check.failed("not supported")
