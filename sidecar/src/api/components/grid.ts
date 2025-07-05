import { createWrapperComponent, createSlottedComponent } from '../utils';

const Grid = createSlottedComponent('Grid', ['searchBarAccessory']);
const GridItem = createSlottedComponent('Grid.Item', ['detail', 'actions']);

const GridSection = createWrapperComponent('Grid.Section');
const GridDropdown = createWrapperComponent('Grid.Dropdown');
const GridDropdownItem = createWrapperComponent('Grid.Dropdown.Item');
const GridDropdownSection = createWrapperComponent('Grid.Dropdown.Section');

const Inset = {
	Small: 'small',
	Medium: 'medium',
	Large: 'large'
} as const;

Object.assign(Grid, {
	Section: GridSection,
	Item: GridItem,
	Dropdown: GridDropdown,
	Inset: Inset
});
Object.assign(GridDropdown, {
	Item: GridDropdownItem,
	Section: GridDropdownSection
});

export { Grid };
