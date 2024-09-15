import json
import logging
import re

import xlwings as xw


class CustomFormatter(logging.Formatter):

    grey = "\x1b[38;20m"
    yellow = "\x1b[33;20m"
    red = "\x1b[31;20m"
    bold_red = "\x1b[31;1m"
    reset = "\x1b[0m"
    format = "%(levelname)s - %(message)s"

    FORMATS = {
        logging.DEBUG: grey + format + reset,
        logging.INFO: grey + format + reset,
        logging.WARNING: yellow + format + reset,
        logging.ERROR: red + format + reset,
        logging.CRITICAL: bold_red + format + reset
    }

    def format(self, record):
        log_fmt = self.FORMATS.get(record.levelno)
        formatter = logging.Formatter(log_fmt)
        return formatter.format(record)

logger = logging.getLogger("ModEx Excel")
logger.setLevel(logging.INFO)
ch = logging.StreamHandler()
ch.setLevel(logging.INFO)
ch.setFormatter(CustomFormatter())
logger.addHandler(ch)

def excel_to_sedaroml(input_filename, output_filename):
    workbook = xw.Book(input_filename)  # connect to a file that is open or in the current working directory

    blocks = {'root': {}}
    index = {
        'Sheet': [],
        'Name': ['ScalarName', 'VectorName', 'MatrixName'],
        'ScalarName': [],
        'VectorName': [],
        'MatrixName': [],
        'Range': ['ScalarRange', 'VectorRange', 'MatrixRange'],
        'ScalarRange': [],
        'VectorRange': [],
        'MatrixRange': [],
    }
    model = {
        'blocks': blocks,
        'index': index,
    }

    for name in workbook.names:
        if re.search(r'(\$?[A-Z]+:\$?[A-Z]+)|(\$?[0-9]+:\$?[0-9]+)', name.refers_to):
            logger.warning(f'Defined names for infinite column or row ranges are not supported. Skipping `{name.name} {name.refers_to}`...')
            continue
        sheet_name, range = name.refers_to.split('=')[-1].split('!')
        sheet = workbook.sheets[sheet_name]
        sheet_id = sheet_name
        refers_to = name.refers_to.split('=')[-1]
        if sheet_id not in blocks:
            blocks[sheet_id] = {
                'id': sheet_id,
                'type': 'Sheet',
                'name': sheet_name,
            }
            index['Sheet'].append(sheet_id)
        if ':' in range:
            block_id = name.name
            blocks[block_id] = {
                'id': block_id,
                'name': name.name,
                'sheet': sheet_id,
                'refers_to': refers_to,
                'value': sheet.range(name.refers_to).value,
            }
            if len(set(re.findall(r'[A-Z]+',range))) == 1 or len(set(re.findall(r'[0-9]+',range))) == 1: # vector
                blocks[block_id]['type'] = 'VectorName'
                index['VectorName'].append(blocks[block_id]['id'])
            else: # matrix
                blocks[block_id]['type'] = 'MatrixName'
                index['MatrixName'].append(blocks[block_id]['id'])
        else: # scalar
            block_id = name.name
            blocks[block_id] = {
                'id': block_id,
                'type': 'ScalarName',
                'name': name.name,
                'sheet': sheet_id,
                'refers_to': refers_to,
                'value': sheet.range(name.refers_to).value,
            }
            index['ScalarName'].append(blocks[block_id]['id'])

        
    # print(json.dumps(model, indent=4))
    with open(output_filename, 'w+') as f:
        json.dump(model, f, indent=4)

def sedaroml_to_excel(input_filename, output_filename):
    logger.debug('Opening Workbook', output_filename)
    workbook = xw.Book(output_filename)  # connect to a file that is open or in the current working directory

    logger.debug('Opening json model', input_filename)
    with open(input_filename, 'r') as f:
        input_model = json.load(f)
    
    logger.debug('Reconciling...')

    for t in ['ScalarName', 'VectorName', 'MatrixName']:
        for id in input_model['index'][t]:
            block = input_model['blocks'][id]
            sheet_block = input_model['blocks'][block['sheet']]
    
            sheet = workbook.sheets[sheet_block['name']]
            logging.debug('Writing Name', block['refers_to'], block['value'])
            sheet.range(block['refers_to']).value = block['value']

def reconcile_diff_to_excel(input_filename, diff_str, output_filename):
    logger.debug('Opening Workbook', output_filename)
    workbook = xw.Book(output_filename)

    logger.debug('Opening json model', input_filename)
    with open(input_filename, 'r') as f:
        input_model = json.load(f)

    logger.debug('Deserializing model diff')
    model_diff = json.loads(diff_str)
    
    logger.debug('Reconciling...')

    # The current ontology does not support adding/removing blocks, so we only need to update the values of changed blocks
    # root can also be ignored
    for id in model_diff['updated_blocks']:
        block = input_model['blocks'][id]
        if block['type'] in ['ScalarName', 'VectorName', 'MatrixName']:
            sheet_block = input_model['blocks'][block['sheet']]

            sheet = workbook.sheets[sheet_block['name']]
            logging.debug('Writing Name', block['refers_to'], block['value'])
            sheet.range(block['refers_to']).value = block['value']