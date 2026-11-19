import { Icon } from './Icon';
import { color } from './tokens';

export type ProfileIconType =
  | 'Gender'
  | 'Country'
  | 'Birthday'
  | 'Pronouns'
  | 'Relationship'
  | 'SexOrientation'
  | 'Language'
  | 'Profession'
  | 'Hobbies'
  | 'RealName'
  | 'EmploymentStatus';

/** The icon of a Profile About field, by its Figma `Type`; Icon/default unless recoloured.
 * Figma Icon/Profile: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1012-265
 */
export function IconProfile({
  type,
  size = 10,
  color: paint = color.icon.default,
}: {
  type: ProfileIconType;
  size?: number;
  color?: string;
}) {
  return <Icon name={`profile${type}`} size={size} color={paint} />;
}
