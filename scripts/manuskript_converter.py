"""Parse a manuskript project and convert it to a format cheese-paper could read"""

import argparse
import logging
import re
import xml
import xml.etree.ElementTree as ET
from pathlib import Path
from uuid import uuid4

import tomli_w


def parse_args():
    """Command line argument parsing"""
    parser = argparse.ArgumentParser("Cheese Paper Manuskript Converter")
    parser.add_argument("manuskript_project", help="Manuskript project to convert")
    parser.add_argument(
        "cheese_paper_project", help="Resulting cheese-paper project directory"
    )
    parser.add_argument(
        "--no-normalize-newlines",
        help=(
            "by default, every instance of one or more newlines will be converted to a "
            "double newline in cheese-paper, this options keeps newlines in place"
        ),
        action="store_false",
        dest="normalize_newlines",
    )
    return parser.parse_args()


def parse_header(header_string: str, normalize_newlines) -> dict:
    """Given a header, get a dictionary nicely representing it"""
    header_values = re.split(r"\n(?!\s)", header_string)

    header_values_split = [val.split(":", 1) for val in header_values if val]

    # we have a bunch of lines that are like '      text', clear the extra whitespace
    # also removes trailing whitespace
    header_values_stripped = {
        key: re.sub(r"(^ +| +$)", "", value, flags=re.MULTILINE)
        for key, value in header_values_split
    }

    if normalize_newlines:
        header_values_processed = {
            key: re.sub(r"\n+", "\n\n", value)
            for key, value in header_values_stripped.items()
        }
        return header_values_processed
    else:
        return header_values_stripped


def parse_character_header(
    contents: str, normalize_newlines: bool
) -> tuple[dict, tuple[str, str]]:
    old_header = parse_header(contents, normalize_newlines)

    old_id = old_header["ID"]
    new_id = str(uuid4())
    header = {
        "name": old_header["Name"],
        "id": new_id,
        "file_type": "character",
        "file_format_version": 1,
    }

    # Manuskript has multiple summaries, combine them all together
    all_summaries = [
        old_header.get("Phrase Summary"),
        old_header.get("Paragraph Summary"),
        old_header.get("Full Summary"),
    ]

    filtered_summaries = [summary for summary in all_summaries if summary]
    combined_summaries = "\n\n----\n\n".join(filtered_summaries)

    if combined_summaries:
        header["summary"] = combined_summaries

    if "Notes" in old_header:
        header["notes"] = old_header["Notes"]

    all_goal = [old_header.get("Motivation"), old_header.get("Goal")]

    filtered_goal = [goal for goal in all_goal if goal]
    combined_goal = "\n\n----\n\n".join(filtered_goal)

    if combined_goal:
        header["goal"] = combined_goal

    all_conflict = [old_header.get("Conflict"), old_header.get("Epiphany")]

    filtered_conflict = [conflict for conflict in all_conflict if conflict]
    combined_conflict = "\n\n----\n\n".join(filtered_conflict)

    if combined_conflict:
        header["conflict"] = combined_conflict

    # anything we copy out above should be listed here
    moved_fields = [
        "Name",
        "ID",
        "Motivation",
        "Goal",
        "Conflict",
        "Epiphany",
        "Phrase Summary",
        "Paragraph Summary",
        "Full Summary",
        "Notes",
    ]

    # there are some fields we just don't have an equivalent for in cheese-paper, we
    # ignore those
    ignored_fields = ["POV", "Color", "Importance"]

    for key, value in old_header.items():
        if key not in moved_fields and key not in ignored_fields:
            header[key] = value

    return (header, (old_id, new_id))


def copy_characters(
    old_directory: Path, new_directory: Path, normalize_newlines: bool
) -> dict[str, str]:
    """Parses and copies all of the characters over"""
    character_id_map = {}
    for character_file in old_directory.glob("*.txt"):
        if character_file.is_dir():
            logging.warning(
                f"found character file that was a directory: {character_file}. skipping"
            )
            continue

        contents = character_file.read_text()

        new_character_header, (old_id, new_id) = parse_character_header(
            contents, normalize_newlines
        )

        character_id_map[old_id] = new_id

        new_directory.joinpath(character_file.stem + ".toml").write_text(
            tomli_w.dumps(new_character_header)
        )

    return character_id_map


def parse_text_header(
    contents: str, normalize_newlines: bool, character_id_map: dict[str, str]
) -> dict:
    """Converts a text file header"""
    old_header = parse_header(contents, normalize_newlines)

    assert old_header["type"] == "md", f"unknown file type {old_header['type']}"

    header = {
        "name": old_header["title"],
        "id": str(uuid4()),
        "file_type": "scene",
        "file_format_version": 1,
    }

    # Manuskript has multiple summaries, combine them all together
    all_summaries = [
        old_header.get("summarySentence"),
        old_header.get("summaryFull"),
    ]
    filtered_summaries = [summary for summary in all_summaries if summary]
    combined_summaries = "\n\n----\n\n".join(filtered_summaries)

    if combined_summaries:
        header["summary"] = combined_summaries

    if "notes" in old_header:
        header["notes"] = old_header["notes"]

    if "compile" in old_header:
        # `"0"` needs to be false, every other int is true (and non-ints are errors)
        header["compile_status"] = int(bool(int(old_header["compile"])))

    # look up what we've changed the IDs to
    if "POV" in old_header:
        new_pov_id = character_id_map.get(old_header["POV"])
        if new_pov_id is not None:
            header["pov"] = f"[|{new_pov_id}]"

    # anything we copy out above should be listed here
    moved_fields = [
        "title",
        "type",
        "summarySentence",
        "summaryFull",
        "POV",
        "notes",
        "compile",
    ]

    # there are some fields we just don't have an equivalent for in cheese-paper, we
    # ignore those
    ignored_fields = ["ID", "label", "status", "setGoal", "charCount"]

    for key, value in old_header.items():
        if key not in moved_fields and key not in ignored_fields:
            header[key] = value

    return header


def parse_folder_header(
    contents: str, normalize_newlines: bool, character_id_map: dict[str, str]
) -> dict:
    """Converts a text folder header"""
    old_header = parse_header(contents, normalize_newlines)

    assert old_header["type"] == "folder", f"unknown folder type {old_header['type']}"

    header = {
        "name": old_header["title"],
        "id": str(uuid4()),
        "file_type": "folder",
        "file_format_version": 1,
    }

    # Manuskript has multiple summaries, combine them all together
    all_summaries = [
        old_header.get("summarySentence"),
        old_header.get("summaryFull"),
    ]
    filtered_summaries = [summary for summary in all_summaries if summary]
    combined_summaries = "\n\n----\n\n".join(filtered_summaries)

    if combined_summaries:
        header["summary"] = combined_summaries

    if "notes" in old_header:
        header["notes"] = old_header["notes"]

    if "compile" in old_header:
        # `"0"` needs to be false, every other int is true (and non-ints are errors)
        header["compile_status"] = int(bool(int(old_header["compile"])))

    # look up what we've changed the IDs to
    if "POV" in old_header:
        new_pov_id = character_id_map.get(old_header["pov"])
        if new_pov_id is not None:
            header["pov"] = f"[|{new_pov_id}]"

    # anything we copy out above should be listed here
    moved_fields = [
        "title",
        "type",
        "summarySentence",
        "summaryFull",
        "POV",
        "notes",
        "compile",
    ]

    # there are some fields we just don't have an equivalent for in cheese-paper, we
    # ignore those
    ignored_fields = ["ID", "label", "status", "setGoal", "charCount"]

    for key, value in old_header.items():
        if key not in moved_fields and key not in ignored_fields:
            header[key] = value

    return header


def copy_text(
    old_directory: Path,
    new_directory: Path,
    normalize_newlines: bool,
    character_id_map: dict[str, str],
):
    """Copies over text file contents"""
    for old_child in old_directory.iterdir():
        new_child_path = new_directory.joinpath(old_child.name)

        if old_child.is_dir():
            # we'll copy folder.txt to metadata.toml when we copy the rest of the children
            new_child_path.mkdir()
            copy_text(old_child, new_child_path, normalize_newlines, character_id_map)
        elif old_child.suffix == ".md":
            contents = old_child.read_text()

            # we do this before stripping any whitespace. putting multiple newlines in a row
            # in the editor will still result in trailing whitespace on disk so we don't need
            # to worry about getting the split in the wrong spot
            sections = contents.split("\n\n\n", 1)

            # since we have a string from reading the file (even if it's empty), we always get back
            # 1 or 2 sections

            text_header = parse_text_header(
                sections[0], normalize_newlines, character_id_map
            )
            if len(sections) == 2:
                if normalize_newlines:
                    body = re.sub(r"\n+", "\n\n", sections[1])
                else:
                    body = sections[1]
            else:
                body = ""

            full_scene_text = tomli_w.dumps(text_header) + "\n\n++++++++\n\n" + body
            new_child_path.write_text(full_scene_text)
        elif old_child.name == "folder.txt":
            contents = old_child.read_text()

            header = parse_folder_header(contents, normalize_newlines, character_id_map)

            new_directory.joinpath("metadata.toml").write_text(tomli_w.dumps(header))


def copy_worldbuilding_elements(
    element: xml.etree.ElementTree.Element, new_directory: Path
):
    """Copies part of the worldbuilding tree into a directory (that should already exist)"""
    for index, child in enumerate(element):
        place_name = child.get("name", "").strip()

        if place_name:
            place_filename = place_name
        else:
            place_filename = "New Place"

        # get a safe version of the name, capped at 30 characters
        filename = re.sub(r"[/\\?%*:|\"<>\x7F\x00-\x1F]", "-", place_filename)[:30]

        filename = filename.replace(" ", "_")

        new_folder = new_directory.joinpath(f"{index:03}-{filename}")

        new_folder.mkdir()

        # now we build the dictionary

        header = {
            "id": str(uuid4()),
            "file_type": "worldbuilding",
            "file_format_version": 1,
        }

        if place_name:
            header["name"] = place_name

        # honestly not sure exactly what this is suppposed to be, adding it as "notes" because
        # that doesn't seem entirely unrelated
        notes = child.get("passion")
        if notes:
            header["notes"] = notes

        description = child.get("description")
        if description:
            header["description"] = description

        connection = child.get("conflict")
        if connection:
            header["connection"] = connection

        # anything we copy out above should be listed here
        moved_fields = [
            "name",
            "description",
            "conflict",
        ]

        # there are some fields we just don't have an equivalent for in cheese-paper, we
        # ignore those
        ignored_fields = ["ID"]

        for key, value in child.items():
            if key not in moved_fields and key not in ignored_fields:
                header[key] = value

        new_folder.joinpath("metadata.toml").write_text(tomli_w.dumps(header))

        copy_worldbuilding_elements(child, new_folder)


def copy_project_metadata(
    old_project_path: Path,
    destination_path: Path,
    normalize_newlines: bool,
) -> bool:
    """
    Copies over project metadata (should be called first), validates if the project seems valid
    """
    # validate that we have a valid project to read from
    infos_path = old_project_path.joinpath("infos.txt")
    summary_path = old_project_path.joinpath("summary.txt")
    if not (infos_path.exists() and summary_path.exists()):
        logging.error(
            f"did not find {infos_path} and {summary_path} so {old_project_path} does not "
            "appear to be a manuskript project. giving up"
        )
        return 1

    project_toml_path = destination_path.joinpath("project.toml")

    header = {
        "id": str(uuid4()),
        "file_type": "scene",
        "file_format_version": 1,
    }

    if infos_path.exists():
        infos_contents = infos_path.read_text()

        infos_dict = parse_header(infos_contents, normalize_newlines)

        if "Title" in infos_dict and "Subtitle" in infos_dict:
            # cheese-paper doesn't really support subtitles, just merge them into one
            header["name"] = infos_dict["Title"] + ": " + infos_dict["Subtitle"]
        elif "Title" in infos_dict:
            # we just have a main title, use that
            header["name"] = infos_dict["Title"]
        elif "Subtitle" in infos_dict:
            # we just have a subtitle, use that
            header["name"] = infos_dict["Subtitle"]

        # we don't process this in cheese-paper right now but it seems like a reasonable
        # future thing so we copy it over.
        # manuskript 0.17.0 encodes this as `Serie` but I have hope that a future version will
        # actually update this
        if "Series" in infos_dict:
            header["series"] = infos_dict["Series"]
        elif "Serie" in infos_dict:
            header["series"] = infos_dict["Serie"]

        # same for volume, we don't process it currently but it's reasonable and will be copied
        if "Volume" in infos_dict:
            header["volume"] = infos_dict["Volume"]

        if "Genre" in infos_dict:
            header["genre"] = infos_dict["Genre"]

        if "Author" in infos_dict:
            header["author"] = infos_dict["Author"]

        if "Email" in infos_dict:
            header["email"] = infos_dict["Email"]

        # anything we copy out above should be listed here
        moved_fields = [
            "Title",
            "Subtitle",
            "Serie",
            "Series",
            "VolumeGenre",
            "Author",
            "Email",
        ]

        # there are some fields we just don't have an equivalent for in cheese-paper, we
        # ignore those
        ignored_fields = ["License"]

        for key, value in infos_dict.items():
            if key not in moved_fields and key not in ignored_fields:
                header[key] = value

    if "name" not in header:
        header["name"] = old_project_path.name

    if summary_path.exists():
        summary_contents = summary_path.read_text()

        summary_dict = parse_header(summary_contents, normalize_newlines)

        # Manuskript has multiple summaries, combine them all together
        all_summaries = [
            summary_dict.get("Situation"),
            summary_dict.get("Sentence"),
            summary_dict.get("Paragraph"),
            summary_dict.get("Page"),
            summary_dict.get("Full"),
        ]
        filtered_summaries = [summary for summary in all_summaries if summary]
        combined_summaries = "\n\n----\n\n".join(filtered_summaries)

        if combined_summaries:
            header["summary"] = combined_summaries

    project_toml_path.write_text(tomli_w.dumps(header))


def main():
    """Do the actual work of parsing the file"""
    args = parse_args()

    old_project_path = Path(args.manuskript_project).expanduser()
    destination_path = Path(args.cheese_paper_project).expanduser()

    if destination_path.exists():
        if destination_path.is_dir():
            if list(destination_path.iterdir()):
                logging.error(
                    f"{destination_path} already exists and is populated. giving up"
                )
                return 1
        else:
            logging.error(f"{destination_path} already exists as a file. giving up")
            return 1
    elif destination_path.parent.exists():
        # this directory doesn't exist but the parent does, we can create it
        destination_path.mkdir()
    else:
        logging.error(
            f"{destination_path} and it's immediate parent do not exist. giving up"
        )
        return 1

    copy_project_metadata(old_project_path, destination_path, args.normalize_newlines)

    # we now have a seemingly valid source project and a valid destination, we can start
    text_path = destination_path.joinpath("text")
    characters_path = destination_path.joinpath("characters")
    worldbuilding_path = destination_path.joinpath("worldbuilding")

    text_path.mkdir()
    characters_path.mkdir()
    worldbuilding_path.mkdir()

    # try to warn users if we're skipping things
    plots_file = old_project_path.joinpath("plots.xml")
    if plots_file.exists():
        tree = ET.parse(plots_file)
        root = tree.getroot()
        if len(root):
            logging.warning(
                "Manuskript project appears to have plots defined. These are "
                "not currently convertible to cheese-paper, skipping."
            )

    # start by copying characters (we need to do this before text)
    old_characters_path = old_project_path.joinpath("characters")
    if old_characters_path.exists():
        if not old_characters_path.is_dir():
            logging.error(
                f"{old_characters_path} exists but is not a directory. giving up"
            )
            return 1

        character_id_map = copy_characters(
            old_characters_path, characters_path, args.normalize_newlines
        )
    else:
        character_id_map = {}

    old_text_path = old_project_path.joinpath("outline")
    if old_text_path.exists():
        if not old_text_path.is_dir():
            logging.error(f"{old_text_path} exists but is not a directory. giving up")
            return 1

        copy_text(old_text_path, text_path, args.normalize_newlines, character_id_map)

    old_worldbuilding_file = old_project_path.joinpath("world.opml")
    if old_worldbuilding_file.exists():
        tree = ET.parse(old_worldbuilding_file)
        body = tree.getroot()[0]  # always the first element
        copy_worldbuilding_elements(body, worldbuilding_path)


if __name__ == "__main__":
    main()
